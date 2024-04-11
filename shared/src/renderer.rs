use crate::prelude::*;
use rayon::prelude::*;
use theframework::prelude::*;

pub struct Renderer {
    pub textures: FxHashMap<Uuid, TheRGBATile>,
    pub tiles: TheFlattenedMap3D<Uuid>,
    pub models: FxHashMap<(i32, i32), TheTimeline>,
    pub position: Vec3f,
    pub hover_pos: Option<Vec3i>,
}

#[allow(clippy::new_without_default)]
impl Renderer {
    pub fn new() -> Self {
        Self {
            textures: FxHashMap::default(),
            tiles: TheFlattenedMap3D::new((0, -1, 0), (80, 2, 80)),
            models: FxHashMap::default(),
            position: Vec3f::zero(),
            hover_pos: None,
        }
    }
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        buffer: &mut TheRGBABuffer,
        region: &Region,
        update: &mut RegionUpdate,
        settings: &mut RegionDrawSettings,
        width: usize,
        height: usize,
        compute_delta: bool,
        palette: &ThePalette,
    ) {
        let _start = self.get_time();

        //let stride = buffer.stride();
        let pixels = buffer.pixels_mut();
        //let height = dim.height;

        let width_f = width as f32;
        let height_f = height as f32;

        let region_height = region.height * region.grid_size;

        let grid_size = region.grid_size as f32;

        if compute_delta {
            update.generate_character_pixel_positions(
                grid_size,
                &self.textures,
                vec2i(width as i32, height as i32),
                region_height,
                settings,
            );
        }

        let mut max_render_distance = 5;
        if let Some(v) = region.regionfx.get(
            str!("Distance / Fog"),
            str!("Maximum Render Distance"),
            &settings.time,
            TheInterpolation::Linear,
        ) {
            if let Some(value) = v.to_i32() {
                max_render_distance = value;
            }
        }

        let mut saturation = None;
        if let Some(v) = region.regionfx.get(
            str!("Saturation"),
            str!("Saturation"),
            &settings.time,
            TheInterpolation::Linear,
        ) {
            if let Some(value) = v.to_f32() {
                saturation = Some(value);
            }
        }

        let mut tilted_iso_alignment = 0;
        if let Some(TheValue::TextList(value, _)) = region.regionfx.get(
            str!("Camera"),
            str!("Tilted Iso Alignment"),
            &settings.time,
            TheInterpolation::Switch,
        ) {
            tilted_iso_alignment = value;
        }

        // Fill the code level with the blocking info and collect lights
        let mut level = Level::new(region.width, region.height, settings.time);
        region.fill_code_level(&mut level, &self.textures, update);

        let (ro, rd, fov, camera_mode, camera_type) = self.create_camera_setup(region, settings);
        let prerender_camera = Camera::prerender(ro, rd, vec2f(width_f, height_f), fov);
        let camera = Camera::new(ro, rd, fov);

        pixels
            .par_rchunks_exact_mut(width * 4)
            .enumerate()
            .for_each(|(j, line)| {
                for (i, pixel) in line.chunks_exact_mut(4).enumerate() {
                    let i = j * width + i;

                    let xx = (i % width) as f32;
                    let yy = (i / width) as f32;

                    let mut ray = if camera_type == CameraType::TiltedIso {
                        camera.create_tilted_isometric_ray_prerendered(
                            vec2f(xx / width_f, yy / height_f),
                            tilted_iso_alignment,
                            &prerender_camera,
                        )
                    } else if camera_mode == CameraMode::Pinhole {
                        camera.create_ray(
                            vec2f(xx / width_f, yy / height_f),
                            vec2f(width_f, height_f),
                            vec2f(1.0, 1.0),
                        )
                    } else {
                        camera.create_ortho_ray_prerendered(
                            vec2f(xx / width_f, yy / height_f),
                            &prerender_camera,
                        )
                    };

                    // In top down view, intersect ray with plane at 1.1 y
                    // to speed up the ray / voxel casting
                    if camera_type != CameraType::FirstPerson {
                        let plane_normal = vec3f(0.0, 1.0, 0.0);
                        let denom = dot(plane_normal, ray.d);

                        if denom.abs() > 0.0001 {
                            let t = dot(vec3f(0.0, 1.0, 0.0) - ray.o, plane_normal) / denom;
                            if t >= 0.0 {
                                ray.o += ray.d * t;
                            }
                        }
                    }

                    pixel.copy_from_slice(&self.render_pixel(
                        ray,
                        region,
                        update,
                        settings,
                        camera_type,
                        &level,
                        &saturation,
                        max_render_distance,
                        palette,
                    ));
                }
            });

        let _stop = self.get_time();
        println!("render time {:?}", _stop - _start);
    }

    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub fn render_pixel(
        &self,
        ray: Ray,
        region: &Region,
        update: &RegionUpdate,
        settings: &RegionDrawSettings,
        camera_type: CameraType,
        level: &Level,
        saturation: &Option<f32>,
        max_render_distance: i32,
        palette: &ThePalette,
    ) -> RGBA {
        let mut color = vec4f(0.0, 0.0, 0.0, 1.0);

        fn equal(l: f32, r: Vec3f) -> Vec3f {
            vec3f(
                if l == r.x { 1.0 } else { 0.0 },
                if l == r.y { 1.0 } else { 0.0 },
                if l == r.z { 1.0 } else { 0.0 },
            )
        }

        let ro = ray.o;
        let rd = ray.d;

        let mut i = floor(ro);
        let mut dist = 0.0;

        let mut normal = Vec3f::zero();
        let srd = signum(rd);

        let rdi = 1.0 / (2.0 * rd);

        let mut key: Vec3<i32>;
        let mut hit = false;

        for _ii in 0..max_render_distance {
            key = Vec3i::from(i);

            if key.y < -1 {
                break;
            }

            if key.y < 1 {
                if let Some(model) = region.models.get(&(key.x, key.y, key.z)) {
                    let mut lro = ray.at(dist);
                    lro -= Vec3f::from(key);
                    lro -= rd * 0.01;

                    let mut r = ray;
                    r.o = lro;

                    if let Some(hit_struct) = model.render(&r, 1.01, i, palette) {
                        color = hit_struct.color;
                        hit = true;
                        //normal = hit_struct.normal;
                        dist += hit_struct.distance;
                        break;
                    }
                /*
                if let Some(hit_struct) = model.hit(&r) {
                    if let Some(tile) = self.tiles.get((key.x, key.y, key.z)) {
                        if let Some(data) = self.textures.get(tile) {
                            let index = settings.anim_counter % data.buffer.len();
                            let mut uv = hit_struct.uv;
                            // TODO apply alpha correctly for WallFX blends
                            let mut alpha: f32 = 1.0;

                            if hit_struct.face != HitFace::YFace && key.y == 0 {
                                // WallFX
                                if let Some(wallfx) = update.wallfx.get(&(key.x, key.z)) {
                                    let mut valid = true;
                                    let mut xx = 0;
                                    let mut yy = 0;
                                    let d = (update.server_tick - wallfx.at_tick) as f32
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
                                            wallfx.fx.apply(&mut xx, &mut yy, &mut alpha, &t, &d);
                                        }
                                    } else if wallfx.fx != WallFX::Normal {
                                        valid = false;
                                    }

                                    if valid {
                                        uv.x += xx as f32 / region.grid_size as f32;
                                        uv.y += yy as f32 / region.grid_size as f32;
                                    } else {
                                        uv = vec2f(-1.0, -1.0);
                                    }
                                }
                            }
                            if let Some(p) = data.buffer[index].at_f_vec4f(uv) {
                                if p[3] == 1.0 {
                                    color = p;
                                    hit = true;
                                    //normal = hit_struct.normal;
                                    dist = hit_struct.distance;
                                    break;
                                }
                            }
                        }
                    }
                    }*/
                }
                // Test against world tiles
                else if let Some(tile) = self.tiles.get((key.x, key.y, key.z)) {
                    let mut uv = self.get_uv(normal, ray.at(dist));
                    //pixel = [(uv.x * 255.0) as u8, (uv.y * 255.0) as u8, 0, 255];
                    if let Some(data) = self.textures.get(tile) {
                        let index = settings.anim_counter % data.buffer.len();

                        // TODO apply alpha correctly for WallFX blends
                        let mut alpha: f32 = 1.0;

                        if key.y == 0 {
                            if let Some(wallfx) = update.wallfx.get(&(key.x, key.z)) {
                                let mut valid = true;
                                let mut xx = 0;
                                let mut yy = 0;
                                let d = (update.server_tick - wallfx.at_tick) as f32
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
                                        wallfx.fx.apply(&mut xx, &mut yy, &mut alpha, &t, &d);
                                    }
                                } else if wallfx.fx != WallFX::Normal {
                                    valid = false;
                                }

                                if valid {
                                    uv.x += xx as f32 / region.grid_size as f32;
                                    uv.y += yy as f32 / region.grid_size as f32;
                                } else {
                                    uv = vec2f(-1.0, -1.0);
                                }
                            }
                        }

                        if !data.billboard {
                            if let Some(p) = data.buffer[index].at_f_vec4f(uv) {
                                //if p[3] == 255 {
                                color = p;
                                hit = true;
                                break;
                                //}
                            }
                        } else {
                            let xx = i.x + 0.5;
                            let zz = i.z + 0.5;

                            let plane_pos = vec3f(xx, 0.5, zz);

                            let mut plane_normal = normalize(plane_pos - ray.o);
                            plane_normal.y = 0.0;
                            let denom = dot(plane_normal, ray.d);

                            if denom > 0.0001 {
                                let t = dot(plane_pos - ray.o, plane_normal) / denom;
                                if t >= 0.0 && !hit || (hit && t < dist) {
                                    let hit_pos = ray.at(t);
                                    if (xx - hit_pos.x).abs() <= 0.5
                                        && (zz - hit_pos.z).abs() <= 0.5
                                        && hit_pos.y >= 0.0
                                        && hit_pos.y <= 1.0
                                    {
                                        #[inline(always)]
                                        fn compute_primary(normal: Vec3f) -> Vec3f {
                                            let a = cross(normal, vec3f(1.0, 0.0, 0.0));
                                            let b = cross(normal, vec3f(0.0, 1.0, 0.0));

                                            let max_ab = if dot(a, a) < dot(b, b) { b } else { a };

                                            let c = cross(normal, vec3f(0.0, 0.0, 1.0));

                                            normalize(if dot(max_ab, max_ab) < dot(c, c) {
                                                c
                                            } else {
                                                max_ab
                                            })
                                        }
                                        let index = settings.anim_counter % data.buffer.len();

                                        let plane_vector_u = compute_primary(plane_normal);
                                        let plane_vector_v = cross(plane_vector_u, rd);

                                        let relative = hit_pos - plane_pos;
                                        let u_dot = dot(relative, plane_vector_u);
                                        let v_dot = dot(relative, plane_vector_v);

                                        let u = 0.5 + u_dot;
                                        let v = 0.5 + v_dot;

                                        //println!("{}, {}", u, v);

                                        let x = (u * data.buffer[index].dim().width as f32) as i32;
                                        let y = ((1.0 - v) * data.buffer[index].dim().height as f32)
                                            as i32;
                                        if let Some(c) = data.buffer[index].at(vec2i(x, y)) {
                                            if c[3] == 255 {
                                                let col = TheColor::from_u8_array(c).to_vec4f();
                                                color = col;
                                                dist = t;
                                                //normal = vec3f(0.0, 0.0, 1.0);
                                                hit = true;
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // if let Some(tile) = self.project.tiles.get(&(key.x, key.y, key.z)) {

            //     let mut lro = ray.at(dist);
            //     lro -= Vec3f::from(key);
            //     lro *= tile.size as f32;
            //     lro = lro - rd * 0.01;

            //     if let Some(mut hit) = tile.dda(&Ray::new(lro, rd)) {
            //         hit.key = key;
            //         hit.hitpoint = ray.at(dist + hit.distance / (tile.size as f32));
            //         hit.distance = dist;
            //         return Some(hit);
            //     }
            // }

            let plain = (1.0 + srd - 2.0 * (ro - i)) * rdi;
            dist = min(plain.x, min(plain.y, plain.z));
            normal = equal(dist, plain) * srd;
            i += normal;
        }

        // Test against characters
        for (pos, tile_id, character_id, _facing) in &update.characters_pixel_pos {
            if camera_type == CameraType::FirstPerson
                && Some(*character_id) == settings.center_on_character
            {
                // Skip the character itself in first person mode.
                continue;
            }

            let xx = pos.x as f32 / region.grid_size as f32 + 0.5;
            let zz = pos.y as f32 / region.grid_size as f32 + 0.5;

            let plane_pos = vec3f(xx, 0.5, zz);

            let mut plane_normal = normalize(plane_pos - ray.o);
            plane_normal.y = 0.0;
            let denom = dot(plane_normal, ray.d);

            if denom > 0.0001 {
                let t = dot(plane_pos - ray.o, plane_normal) / denom;
                if t >= 0.0 && !hit || (hit && t < dist) {
                    let hit_point = ray.at(t);
                    if (xx - hit_point.x).abs() <= 0.5
                        && (zz - hit_point.z).abs() <= 0.5
                        && hit_point.y >= 0.0
                        && hit_point.y <= 1.0
                    {
                        if let Some(data) = self.textures.get(tile_id) {
                            #[inline(always)]
                            fn compute_primary(normal: Vec3f) -> Vec3f {
                                let a = cross(normal, vec3f(1.0, 0.0, 0.0));
                                let b = cross(normal, vec3f(0.0, 1.0, 0.0));

                                let max_ab = if dot(a, a) < dot(b, b) { b } else { a };

                                let c = cross(normal, vec3f(0.0, 0.0, 1.0));

                                normalize(if dot(max_ab, max_ab) < dot(c, c) {
                                    c
                                } else {
                                    max_ab
                                })
                            }
                            let index = settings.anim_counter % data.buffer.len();

                            // let plane_vector_u = normalize(cross(rd, vec3f(1.0, 0.0, 0.0)));
                            // let plane_vector_v = normalize(cross(rd, vec3f(0.0, 1.0, 0.0)));

                            let plane_vector_u = compute_primary(plane_normal);
                            let plane_vector_v = cross(plane_vector_u, rd);

                            let relative = hit_point - plane_pos;
                            let u_dot = dot(relative, plane_vector_u);
                            let v_dot = dot(relative, plane_vector_v);

                            let u = 0.5 + u_dot;
                            let v = 0.5 + v_dot;

                            //println!("{}, {}", u, v);

                            let x = (u * data.buffer[index].dim().width as f32) as i32;
                            let y = ((1.0 - v) * data.buffer[index].dim().height as f32) as i32;
                            if let Some(c) = data.buffer[index].at(vec2i(x, y)) {
                                if c[3] == 255 {
                                    let col = TheColor::from_u8_array(c).to_vec4f();
                                    color = col;
                                    dist = t;
                                    //normal = vec3f(0.0, 0.0, 1.0);
                                    hit = true;
                                }
                            }
                        }
                    }
                }
            }
        }

        // Light Sampling
        //
        if hit {
            let daylight = vec4f(
                settings.daylight.x,
                settings.daylight.y,
                settings.daylight.z,
                1.0,
            );

            if level.lights.is_empty() {
                color *= daylight;
            } else {
                // Sample the lights
                let mut total_light = Vec3f::new(0.0, 0.0, 0.0);
                for (light_grid, light_coll) in &level.lights {
                    let light_pos = Vec2f::from(*light_grid) + vec2f(0.5, 0.5);
                    let light_max_distance =
                        light_coll.get_i32_default("Maximum Distance", 10) as f32;
                    let mut light_strength = light_coll.get_f32_default("Emission Strength", 1.0);
                    let light_sampling_off = light_coll.get_f32_default("Sample Offset", 0.5);
                    let light_samples = light_coll.get_i32_default("Samples #", 5) as usize;
                    let light_color_type = light_coll.get_i32_default("Light Color", 0);
                    let light_color = light_coll.get_float3_default("Color", vec3f(1.0, 1.0, 1.0));
                    let light_limiter = light_coll.get_i32_default("Limit Direction", 0);

                    if light_color_type == 1 {
                        light_strength = daylight.x;
                    }

                    let ro = ray.at(dist);

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

                    let y = 0.5;
                    let offsets = [
                        ro,
                        ro - vec3f(0.0, y, light_sampling_off),
                        ro - vec3f(light_sampling_off, y, 0.0),
                        ro + vec3f(light_sampling_off, y, 0.0),
                        ro + vec3f(0.0, y, light_sampling_off),
                        ro - vec3f(light_sampling_off, y, light_sampling_off),
                        ro + vec3f(light_sampling_off, y, light_sampling_off),
                    ];

                    for s in offsets.iter().take(light_samples) {
                        let ro = s;

                        let mut light_dir = vec3f(light_pos.x, y, light_pos.y) - ro;
                        let light_dist = length(light_dir);

                        if light_dist < light_max_distance {
                            light_dir = normalize(light_dir);

                            let mut t = 0.0;
                            let max_t = light_dist;

                            let mut hit = false;

                            while t < max_t {
                                let pos = ro + light_dir * t;
                                let tile = vec2i(pos.x as i32, pos.z as i32);

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
                                let mut light =
                                    Vec3f::from(intensity * light_strength / light_samples as f32);
                                if light_color_type == 0 {
                                    light *= light_color
                                }
                                total_light += light;
                            }
                        }
                    }
                }

                color = clamp(
                    color * daylight
                        + color * vec4f(total_light.x, total_light.y, total_light.z, 1.0),
                    color * daylight,
                    color,
                );
            }
        }

        if let Some(saturation) = saturation {
            let mut hsl = TheColor::from_vec4f(color).as_hsl();
            hsl.y *= saturation;
            color = TheColor::from_hsl(hsl.x * 360.0, hsl.y.clamp(0.0, 1.0), hsl.z).to_vec4f();
        }

        if let Some(hover) = self.hover_pos {
            let hp = ray.at(dist);

            if hp.x as i32 == hover.x && hp.z as i32 == hover.z {
                color = TheColor::from_vec4f(color)
                    .mix(&TheColor::white(), 0.5)
                    .to_vec4f();
            }
        }

        TheColor::from_vec4f(color).to_u8_array()
    }

    #[inline(always)]
    pub fn get_uv(&self, normal: Vec3f, hp: Vec3f) -> Vec2f {
        // Calculate the absolute values of the normal components
        let abs_normal = abs(normal);

        // Determine which face of the cube was hit based on the maximum component of the normal
        let face_index = if abs_normal.x > abs_normal.y {
            if abs_normal.x > abs_normal.z {
                0 // X-axis face
            } else {
                2 // Z-axis face
            }
        } else if abs_normal.y > abs_normal.z {
            1 // Y-axis face
        } else {
            2 // Z-axis face
        };

        // Calculate UV coordinates based on the face
        match face_index {
            0 => Vec2f::new(frac(hp.z), 1.0 - frac(hp.y)), // X-axis face
            1 => Vec2f::new(frac(hp.x), frac(hp.z)),       // Y-axis face
            2 => Vec2f::new(frac(hp.x), 1.0 - frac(hp.y)), // Z-axis face
            _ => Vec2f::zero(),
        }
    }

    pub fn set_region(&mut self, region: &Region) {
        self.tiles.clear();
        //self.models = region.models.clone();

        for (pos, tile) in &region.tiles {
            for i in 0..tile.layers.len() {
                if i == 0 {
                    if let Some(tile_uuid) = tile.layers[i] {
                        self.tiles.set((pos.0, -1, pos.1), tile_uuid);
                    }
                } else if i == 1 {
                    if let Some(tile_uuid) = tile.layers[i] {
                        self.tiles.set((pos.0, 0, pos.1), tile_uuid);
                    }
                } else if i == 2 {
                    if let Some(tile_uuid) = tile.layers[i] {
                        self.tiles.set((pos.0, 1, pos.1), tile_uuid);
                    }
                }
            }
        }
    }

    pub fn set_textures(&mut self, tiles: FxHashMap<Uuid, TheRGBATile>) {
        self.textures = tiles;
    }

    pub fn set_position(&mut self, position: Vec3f) {
        self.position = position;
    }

    /// Create the camera setup.
    pub fn create_camera_setup(
        &mut self,
        region: &Region,
        settings: &mut RegionDrawSettings,
    ) -> (Vec3f, Vec3f, f32, CameraMode, CameraType) {
        let mut position = self.position;
        let mut facing = vec3f(0.0, 0.0, -1.0);
        if settings.center_on_character.is_some() {
            position = settings.center_3d + self.position;
            facing = settings.facing_3d;
        }

        // Get the camera settings

        let camera_type = region.camera_type;
        let mut first_person_height = 0.5;
        let mut top_down_height = 4.0;
        let mut top_down_x_offset = -5.0;
        let mut top_down_z_offset = 5.0;
        let mut first_person_fov = 70.0;
        let top_down_fov = 75.0;
        let tilted_iso_height = 3.0;
        let tilted_iso_fov = 75.0;

        if let Some(v) = region.regionfx.get(
            str!("Camera"),
            str!("First Person FoV"),
            &settings.time,
            TheInterpolation::Linear,
        ) {
            if let Some(value) = v.to_f32() {
                first_person_fov = value;
            }
        }

        if let Some(v) = region.regionfx.get(
            str!("Camera"),
            str!("First Person Height"),
            &settings.time,
            TheInterpolation::Linear,
        ) {
            if let Some(value) = v.to_f32() {
                first_person_height = value;
            }
        }

        if let Some(v) = region.regionfx.get(
            str!("Camera"),
            str!("Top Down Height"),
            &settings.time,
            TheInterpolation::Linear,
        ) {
            if let Some(value) = v.to_f32() {
                top_down_height = value;
            }
        }

        if let Some(v) = region.regionfx.get(
            str!("Camera"),
            str!("Top Down X Offset"),
            &settings.time,
            TheInterpolation::Linear,
        ) {
            if let Some(value) = v.to_f32() {
                top_down_x_offset = value;
            }
        }

        if let Some(v) = region.regionfx.get(
            str!("Camera"),
            str!("Top Down Z Offset"),
            &settings.time,
            TheInterpolation::Linear,
        ) {
            if let Some(value) = v.to_f32() {
                top_down_z_offset = value;
            }
        }

        // Camera

        let mut ro = vec3f(position.x + 0.5, 0.5, position.z + 0.5);
        let rd;
        let fov;
        let mut camera_mode = CameraMode::Pinhole;

        if camera_type == CameraType::TopDown {
            rd = ro;
            ro.y = top_down_height;
            ro.x += top_down_x_offset;
            ro.z += top_down_z_offset;
            fov = top_down_fov;
            camera_mode = CameraMode::Orthogonal;
        } else if camera_type == CameraType::FirstPerson {
            // First person
            ro.y = first_person_height;
            rd = ro + facing * 2.0;
            fov = first_person_fov;
        } else {
            // Tilted iso
            rd = ro;
            ro.y = tilted_iso_height;
            ro.z += 1.0;
            fov = tilted_iso_fov;
            camera_mode = CameraMode::Orthogonal;
        }

        fn transform_zoom(z: f32, power: f32) -> f32 {
            1.0 + (z - 1.0).powf(power) / 10.0
        }

        (
            ro,
            rd,
            fov / (transform_zoom(region.zoom, 2.0)),
            camera_mode,
            camera_type,
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

    /// Returns the terrain hit position at the given screen coordinate (if any).
    pub fn get_hit_position_at(
        &mut self,
        screen_coord: Vec2i,
        region: &Region,
        settings: &mut RegionDrawSettings,
        width: usize,
        height: usize,
    ) -> Option<Vec3i> {
        let (ro, rd, fov, camera_mode, camera_type) = self.create_camera_setup(region, settings);

        let width_f = width as f32;
        let height_f = height as f32;

        let mut tilted_iso_alignment = 0;
        if let Some(TheValue::TextList(value, _)) = region.regionfx.get(
            str!("Camera"),
            str!("Tilted Iso Alignment"),
            &settings.time,
            TheInterpolation::Switch,
        ) {
            tilted_iso_alignment = value;
        }

        let camera = Camera::new(ro, rd, fov);
        let ray = if camera_type == CameraType::TiltedIso {
            camera.create_tilted_isometric_ray(
                vec2f(
                    screen_coord.x as f32 / width_f,
                    1.0 - screen_coord.y as f32 / height_f,
                ),
                vec2f(width_f, height_f),
                vec2f(1.0, 1.0),
                tilted_iso_alignment,
            )
        } else if camera_mode == CameraMode::Pinhole {
            camera.create_ray(
                vec2f(
                    screen_coord.x as f32 / width_f,
                    1.0 - screen_coord.y as f32 / height_f,
                ),
                vec2f(width_f, height_f),
                vec2f(1.0, 1.0),
            )
        } else {
            camera.create_ortho_ray(
                vec2f(
                    screen_coord.x as f32 / width_f,
                    1.0 - screen_coord.y as f32 / height_f,
                ),
                vec2f(width_f, height_f),
                vec2f(1.0, 1.0),
            )
        };

        fn equal(l: f32, r: Vec3f) -> Vec3f {
            vec3f(
                if l == r.x { 1.0 } else { 0.0 },
                if l == r.y { 1.0 } else { 0.0 },
                if l == r.z { 1.0 } else { 0.0 },
            )
        }

        let ro = ray.o;
        let rd = ray.d;

        let mut i = floor(ro);
        let mut dist;

        let mut normal;
        let srd = signum(rd);

        let rdi = 1.0 / (2.0 * rd);

        let mut key: Vec3<i32>;

        for _ii in 0..50 {
            key = Vec3i::from(i);

            if key.y < -1 {
                break;
            }

            if region.models.get(&(key.x, key.y, key.z)).is_some() {
                return Some(vec3i(key.x, key.y, key.z));
            }
            // Test against world tiles
            if self.tiles.get((key.x, key.y, key.z)).is_some() {
                return Some(vec3i(key.x, 0, key.z));
            }

            let plain = (1.0 + srd - 2.0 * (ro - i)) * rdi;
            dist = min(plain.x, min(plain.y, plain.z));
            normal = equal(dist, plain) * srd;
            i += normal;
        }
        None
    }
}

/*
// Alternative to billboarding: Auto face to North / South or East / West
if facing.y.abs() > facing.x.abs() {
    // Character is north / south aligned

    let plane_normal = vec3f(0.0, 0.0, 1.0);
    let denom = dot(plane_normal, ray.d);

    let xx = pos.x as f32 / region.grid_size as f32 + 0.5;
    let zz = pos.y as f32 / region.grid_size as f32 + 0.5;

    if denom.abs() > 0.0001 {
        let t = dot(vec3f(xx, 0.0, zz) - ray.o, plane_normal) / denom;
        if t >= 0.0 && !hit || (hit && t < dist) {
            let hit = ray.at(t);
            if (xx - hit.x).abs() <= 0.5
                && (zz - hit.z).abs() <= 0.5
                && hit.y >= 0.0
                && hit.y <= 1.0
            {
                if let Some(data) = self.textures.get(tile_id) {
                    //color.x = 1.0;
                    let index = settings.anim_counter % data.buffer.len();
                    let x = ((hit.x - xx + 0.5) * data.buffer[index].dim().width as f32)
                        as i32;
                    let y =
                        ((1.0 - hit.y) * data.buffer[index].dim().height as f32) as i32;
                    if let Some(c) = data.buffer[index].at(vec2i(x, y)) {
                        if c[3] == 255 {
                            let col = TheColor::from_u8_array(c).to_vec4f();
                            color = col;
                            dist = t;
                            normal = vec3f(0.0, 0.0, 1.0);
                        }
                    }
                }
            }
        }
    }
} else {
    // Character is east / west aligned

    let plane_normal = vec3f(1.0, 0.0, 0.0);
    let denom = dot(plane_normal, ray.d);

    let xx = pos.x as f32 / region.grid_size as f32 + 0.5;
    let zz = pos.y as f32 / region.grid_size as f32 + 0.5;

    if denom.abs() > 0.0001 {
        let t = dot(vec3f(xx, 0.0, zz) - ray.o, plane_normal) / denom;
        if t >= 0.0 && !hit || (hit && t < dist) {
            let hit = ray.at(t);
            if (xx - hit.x).abs() <= 0.5
                && (zz - hit.z).abs() <= 0.5
                && hit.y >= 0.0
                && hit.y <= 1.0
            {
                if let Some(data) = self.textures.get(tile_id) {
                    //color.x = 1.0;
                    let index = settings.anim_counter % data.buffer.len();
                    let x = ((hit.z - zz + 0.5) * data.buffer[index].dim().width as f32)
                        as i32;
                    let y =
                        ((1.0 - hit.y) * data.buffer[index].dim().height as f32) as i32;
                    if let Some(c) = data.buffer[index].at(vec2i(x, y)) {
                        if c[3] == 255 {
                            let col = TheColor::from_u8_array(c).to_vec4f();
                            color = col;
                            dist = t;
                            normal = vec3f(0.0, 0.0, 1.0);
                        }
                    }
                }
            }
        }
    }
    }*/
