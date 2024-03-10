use crate::prelude::*;
use rayon::prelude::*;
use theframework::prelude::*;

pub struct Renderer {
    pub textures: FxHashMap<Uuid, TheRGBATile>,
    pub tiles: TheFlattenedMap3D<Uuid>,
    pub models: FxHashMap<(i32, i32), TheTimeline>,
    pub position: Vec3f,
}

#[allow(clippy::new_without_default)]
impl Renderer {
    pub fn new() -> Self {
        Self {
            textures: FxHashMap::default(),
            tiles: TheFlattenedMap3D::new((0, -1, 0), (80, 2, 80)),
            models: FxHashMap::default(),
            position: Vec3f::zero(),
        }
    }

    pub fn render(
        &mut self,
        buffer: &mut TheRGBABuffer,
        region: &Region,
        update: &mut RegionUpdate,
        settings: &mut RegionDrawSettings,
        width: usize,
        height: usize,
        compute_delta: bool,
    ) {
        let start = self.get_time();

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

        let mut models = FxHashMap::default();
        for (pos, timeline) in &self.models {
            //self.models.set((pos.0, 0, pos.1), timeline.clone());
            let timeline = ModelFX::parse_timeline(&settings.time, timeline);
            models.insert(*pos, timeline);
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

        let (ro, rd, fov, camera_mode, camera_type) = self.create_camera_setup(region, settings);

        pixels
            .par_rchunks_exact_mut(width * 4)
            .enumerate()
            .for_each(|(j, line)| {
                for (i, pixel) in line.chunks_exact_mut(4).enumerate() {
                    let i = j * width + i;

                    let xx = (i % width) as f32;
                    let yy = (i / width) as f32;

                    let camera = Camera::new(ro, rd, fov);
                    let ray = if camera_mode == CameraMode::Pinhole {
                        camera.create_ray(
                            vec2f(xx / width_f, yy / height_f),
                            vec2f(width_f, height_f),
                            vec2f(0.0, 0.0),
                        )
                    } else {
                        camera.create_ortho_ray(
                            vec2f(xx / width_f, yy / height_f),
                            vec2f(width_f, height_f),
                            vec2f(0.0, 0.0),
                        )
                    };

                    pixel.copy_from_slice(&self.render_pixel(
                        ray,
                        region,
                        update,
                        settings,
                        camera_type,
                        &models,
                        &saturation,
                    ));
                }
            });

        let _stop = self.get_time();
        println!("render time {:?}", _stop - start);
    }

    #[inline(always)]
    pub fn render_pixel(
        &self,
        ray: Ray,
        region: &Region,
        update: &RegionUpdate,
        settings: &RegionDrawSettings,
        camera_type: CameraType,
        models: &FxHashMap<(i32, i32), Vec<ModelFX>>,
        saturation: &Option<f32>,
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

        for _ii in 0..30 {
            key = Vec3i::from(i);

            if let Some(models) = models.get(&(key.x, key.z)) {
                let mut lro = ray.at(dist);
                lro -= Vec3f::from(key);
                //lro *= tile.size as f32;
                lro = lro - rd * 0.01;

                let mut r = ray.clone();
                r.o = lro;

                if let Some(hit_struct) = ModelFX::hit_array(&r, models) {
                    if let Some(tile) = self.tiles.get((key.x, key.y, key.z)) {
                        if let Some(data) = self.textures.get(tile) {
                            let index = settings.anim_counter % data.buffer.len();
                            if let Some(p) = data.buffer[index].at_f_vec4f(hit_struct.uv) {
                                color = p;
                                hit = true;
                                normal = hit_struct.normal;
                                dist = hit_struct.distance;
                                break;
                            }
                        }
                    }
                }
            } else
            // Test against world tiles
            if let Some(tile) = self.tiles.get((key.x, key.y, key.z)) {
                let uv = self.get_uv(normal, ray.at(dist));
                //pixel = [(uv.x * 255.0) as u8, (uv.y * 255.0) as u8, 0, 255];
                if let Some(data) = self.textures.get(tile) {
                    let index = settings.anim_counter % data.buffer.len();
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
                                    let y =
                                        ((1.0 - v) * data.buffer[index].dim().height as f32) as i32;
                                    if let Some(c) = data.buffer[index].at(vec2i(x, y)) {
                                        if c[3] == 255 {
                                            let col = TheColor::from_u8_array(c).to_vec4f();
                                            color = col;
                                            dist = t;
                                            normal = vec3f(0.0, 0.0, 1.0);
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
            if camera_type == CameraType::FirstPerson {
                if Some(*character_id) == settings.center_on_character {
                    // Skip the character itself in first person mode.
                    continue;
                }
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
                    let hit = ray.at(t);
                    if (xx - hit.x).abs() <= 0.5
                        && (zz - hit.z).abs() <= 0.5
                        && hit.y >= 0.0
                        && hit.y <= 1.0
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

                            let relative = hit - plane_pos;
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
                                    normal = vec3f(0.0, 0.0, 1.0);
                                }
                            }
                        }
                    }
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
        }

        if hit {
            // We hit something
            color.x *= settings.daylight.x;
            color.y *= settings.daylight.y;
            color.z *= settings.daylight.z;
        }

        if let Some(saturation) = saturation {
            let mut hsl = TheColor::from_vec4f(color).as_hsl();
            hsl.y *= saturation;
            color = TheColor::from_hsl(hsl.x * 360.0, hsl.y.clamp(0.0, 1.0), hsl.z).to_vec4f();
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
        self.models = region.models.clone();

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
            position = settings.center_3d;
            facing = settings.facing_3d;
        }

        // Get the camera settings

        let mut camera_type = CameraType::TopDown;
        let mut top_down_camera_mode = CameraMode::Orthogonal;
        let mut first_person_height = 0.5;
        let mut top_down_height = 4.0;
        let mut top_down_x_offset = -5.0;
        let mut top_down_z_offset = 5.0;
        let mut first_person_fov = 70.0;
        let mut top_down_fov = 55.0;

        if let Some(TheValue::TextList(value, _)) = region.regionfx.get(
            str!("Camera"),
            str!("Camera Type"),
            &settings.time,
            TheInterpolation::Linear,
        ) {
            if value == 0 {
                camera_type = CameraType::FirstPerson;
            }
        }

        if let Some(TheValue::TextList(value, _)) = region.regionfx.get(
            str!("Camera"),
            str!("Top Down Camera"),
            &settings.time,
            TheInterpolation::Switch,
        ) {
            if value == 0 {
                top_down_camera_mode = CameraMode::Pinhole;
            }
        }

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
            str!("Top Down FoV"),
            &settings.time,
            TheInterpolation::Linear,
        ) {
            if let Some(value) = v.to_f32() {
                top_down_fov = value;
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
            camera_mode = top_down_camera_mode;
        } else {
            // First person
            ro.y = first_person_height;
            rd = ro + facing * 2.0;
            fov = first_person_fov;
        }

        (ro, rd, fov, camera_mode, camera_type)
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
