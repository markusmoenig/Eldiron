use crate::prelude::*;
use rayon::prelude::*;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use theframework::prelude::*;

pub struct Renderer {
    pub textures: FxHashMap<Uuid, TheRGBATile>,
    pub materials: IndexMap<Uuid, MaterialFXObject>,
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
            materials: IndexMap::default(),
            tiles: TheFlattenedMap3D::new((0, -1, 0), (80, 2, 80)),
            models: FxHashMap::default(),
            position: Vec3f::zero(),
            hover_pos: None,
        }
    }

    pub fn prerender_rtree(
        &mut self,
        prerendered: &mut PreRendered,
        region: &Region,
        settings: &mut RegionDrawSettings,
    ) {
        let _start = self.get_time();

        let width = prerendered.albedo.dim().width as usize;
        let height = prerendered.albedo.dim().height as usize;

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

        let (ro, rd, fov, _, camera_type) = self.create_camera_setup(region, settings);
        let camera = Camera::new(ro, rd, fov);

        // --
        let mut tiles = vec![];
        let w = prerendered.albedo.dim().width / region.grid_size;
        let h = prerendered.albedo.dim().height / region.grid_size;
        for x in 0..w {
            for y in 0..h {
                let tile = Vec2i::new(x, y);
                tiles.push(tile);
            }
        }

        let prerendered_mutex = Arc::new(Mutex::new(prerendered));

        let _start = self.get_time();

        // Temporary array to store the rtree values, we bulk_load the content at the end
        let dest_tree = vec![];
        let dest_tree_mutex = Arc::new(Mutex::new(dest_tree));

        tiles.par_iter_mut().for_each(|tile| {
            let mut tree = vec![];

            for h in 0..region.grid_size {
                for w in 0..region.grid_size {
                    let x = tile.x * region.grid_size + w;
                    let y = tile.y * region.grid_size + h;
                    let xx = x as f32;
                    let yy = y as f32;

                    let ray = if camera_type == CameraType::TiltedIso {
                        camera.create_tilted_isometric_ray2(
                            vec2f(xx / width_f, (height_f - yy) / height_f),
                            vec2f(width_f, height_f),
                            vec2f(region.width as f32, region.height as f32),
                            vec2f(1.0, 1.0),
                            tilted_iso_alignment,
                        )
                    } else {
                        camera.create_ortho_ray2(
                            vec2f(xx / width_f, (height_f - yy) / height_f),
                            vec2f(width_f, height_f),
                            vec2f(region.width as f32, region.height as f32),
                            vec2f(1.0, 1.0),
                        )
                    };

                    let plane_normal = vec3f(0.0, 1.0, 0.0);
                    let denom = dot(plane_normal, ray.d);

                    if denom.abs() > 0.0001 {
                        let t = dot(vec3f(0.0, 0.0, 0.0) - ray.o, plane_normal) / denom;
                        if t >= 0.0 {
                            let p = ray.o + ray.d * t;
                            tree.push(PreRenderedData {
                                location: (p.x, p.z),
                                pixel_location: (xx as i32, (yy) as i32),
                            });
                        }
                    }
                }
            }

            let mut dest_tree_mutex = dest_tree_mutex.lock().unwrap();
            dest_tree_mutex.append(&mut tree);

            std::thread::yield_now();
        });

        let mut prerendered = prerendered_mutex.lock().unwrap();

        prerendered.tree = RTree::bulk_load(
            Arc::try_unwrap(dest_tree_mutex)
                .unwrap()
                .into_inner()
                .unwrap(),
        );

        let _stop = self.get_time();
        //println!("render time {:?}", _stop - _start);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn prerender(
        &mut self,
        prerendered: &mut PreRendered,
        region: &Region,
        settings: &mut RegionDrawSettings,
        palette: &ThePalette,
        sender: mpsc::Sender<PreRenderResult>,
    ) -> bool {
        // --
        let mut tiles = vec![];

        // Add all tiles which do not have all samples
        let w = prerendered.albedo.dim().width / region.grid_size;
        let h = prerendered.albedo.dim().height / region.grid_size;
        for x in 0..w {
            for y in 0..h {
                let tile = Vec2i::new(x, y);
                if let Some(samples) = prerendered.tile_samples.get(&tile) {
                    if (*samples as i32) < region.pathtracer_samples {
                        tiles.push(tile);
                    }
                } else {
                    tiles.push(tile);
                }
            }
        }

        // Finished ?
        if tiles.is_empty() {
            return false;
        }

        // println!("tiles {}", tiles.len());

        let _start = self.get_time();

        let width = prerendered.albedo.dim().width as usize;
        let height = prerendered.albedo.dim().height as usize;

        let width_f = width as f32;
        let height_f = height as f32;

        settings.pbr = false;
        if let Some(v) = region.regionfx.get(
            str!("Renderer"),
            str!("Shading"),
            &settings.time,
            TheInterpolation::Linear,
        ) {
            if let Some(value) = v.to_i32() {
                if value == 1 {
                    settings.pbr = true;
                }
            }
        }

        let mut max_render_distance = 20;
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

        let mut tilted_iso_alignment = 0;
        if let Some(TheValue::TextList(value, _)) = region.regionfx.get(
            str!("Camera"),
            str!("Tilted Iso Alignment"),
            &settings.time,
            TheInterpolation::Switch,
        ) {
            tilted_iso_alignment = value;
        }

        let update = RegionUpdate::default();

        // Fill the code level with the blocking info and collect lights
        let mut level = Level::new(region.width, region.height, settings.time);
        region.fill_code_level(&mut level, &self.textures, &update);

        let (ro, rd, fov, _, camera_type) = self.create_camera_setup(region, settings);
        //let prerender_camera = Camera::prerender(ro, rd, vec2f(width_f, height_f), fov);
        let camera = Camera::new(ro, rd, fov);

        let prerendered_mutex = Arc::new(Mutex::new(prerendered));

        let _start = self.get_time();

        tiles.par_iter_mut().for_each(|tile| {
            let mut buffer = TheRGBBuffer::new(TheDim::sized(region.grid_size, region.grid_size));
            let mut sky_abso_buffer =
                TheRGBBuffer::new(TheDim::sized(region.grid_size, region.grid_size));
            let mut rng = rand::thread_rng();

            let mut distance_buffer: TheFlattenedMap<f32> =
                TheFlattenedMap::new(region.grid_size, region.grid_size);

            let mut lights_buffer: TheFlattenedMap<Vec<PreRenderedLight>> =
                TheFlattenedMap::new(region.grid_size, region.grid_size);

            let mut tile_is_empty = true;

            for h in 0..region.grid_size {
                for w in 0..region.grid_size {
                    let x = tile.x * region.grid_size + w;
                    let y = tile.y * region.grid_size + h;
                    let xx = x as f32;
                    let yy = y as f32;

                    // Pathtracer
                    // Based on https://www.shadertoy.com/view/Dtl3WS

                    let mut empty_pixel = false;
                    let mut color = Vec3f::zero();
                    let mut sky_abso = Vec3f::zero();
                    let mut distance: f32 = 0.0;
                    let mut lights: Vec<PreRenderedLight> = vec![];

                    for sample in 0..1 {
                        let mut ray = if camera_type == CameraType::TiltedIso {
                            camera.create_tilted_isometric_ray2(
                                vec2f(xx / width_f, (height_f - yy) / height_f),
                                vec2f(width_f, height_f),
                                vec2f(region.width as f32, region.height as f32),
                                vec2f(rng.gen(), rng.gen()),
                                tilted_iso_alignment,
                            )
                        } else {
                            camera.create_ortho_ray2(
                                vec2f(xx / width_f, (height_f - yy) / height_f),
                                vec2f(width_f, height_f),
                                vec2f(region.width as f32, region.height as f32),
                                vec2f(rng.gen(), rng.gen()),
                            )
                        };

                        let plane_normal = vec3f(0.0, 1.0, 0.0);
                        let denom = dot(plane_normal, ray.d);

                        if denom.abs() > 0.0001 {
                            let t = dot(vec3f(0.0, 1.1, 0.0) - ray.o, plane_normal) / denom;
                            if t >= 0.0 {
                                ray.o += ray.d * t;
                            }
                        }

                        if true {
                            let mut acc = Vec3f::zero();
                            let mut mask = Vec3f::one();
                            let mut dist = 0.0;

                            for depth in 0..8 {
                                if let Some(hit) = self.prerender_pixel(
                                    ray,
                                    region,
                                    &update,
                                    settings,
                                    max_render_distance,
                                    palette,
                                    &mut rng,
                                ) {
                                    if depth == 0 {
                                        dist = hit.distance;
                                    }

                                    let x = hit.hit_point;
                                    let n = hit.normal;
                                    let nl = n * signum(-dot(n, ray.d));

                                    let e = 1.0;
                                    let roughness = hit.roughness;
                                    let alpha = roughness * roughness;
                                    let metallic = hit.metallic;
                                    let reflectance = hit.reflectance;
                                    let color = hit.albedo;

                                    //let mut brdf = Vec3f::zero();

                                    if rng.gen::<f32>() < reflectance {
                                        // Sphere s = lightSourceVolume;
                                        // int i = 6;

                                        // vec3 l0 = s.p - x;
                                        // float cos_a_max = sqrt(1. - clamp(s.r * s.r / dot(l0, l0), 0., 1.));
                                        // float cosa = mix(cos_a_max, 1., rand());
                                        // vec3 l = jitter(l0, 2.*PI*rand(), sqrt(1. - cosa*cosa), cosa);

                                        // if (intersect(Ray(x, l), t, s, id) == i) {
                                        //     float omega = 2. * PI * (1. - cos_a_max);
                                        //     brdf += (s.e * clamp(ggx(nl, r.d, l, roughness, metallic),0.,1.) * omega) / PI;
                                        // }

                                        // Direct light sampling
                                        if depth == 0 {
                                            for (light_grid, light) in &level.lights {
                                                let light_dist =
                                                    length(Vec2f::from(*light_grid - *tile));

                                                if light_dist < light.max_distance {
                                                    let light_pos = vec3f(
                                                        light_grid.x as f32 + 0.5,
                                                        0.5,
                                                        light_grid.y as f32 + 0.5,
                                                    );
                                                    let l0 = light_pos - x;

                                                    let lr = light.max_distance;
                                                    let cos_a_max = sqrt(
                                                        1. - clamp(lr * lr / dot(l0, l0), 0., 1.),
                                                    );
                                                    let cosa = lerp(cos_a_max, 1., rng.gen());
                                                    let l = jitter(
                                                        l0,
                                                        2. * f32::pi() * rng.gen::<f32>(),
                                                        sqrt(1. - cosa * cosa),
                                                        cosa,
                                                    );

                                                    if self.shadow_ray(
                                                        Ray::new(x, l0),
                                                        Vec3i::from(light_pos),
                                                        light,
                                                        region,
                                                        &update,
                                                        settings,
                                                    ) {
                                                        let omega =
                                                            2.0 * f32::pi() * (1.0 - cos_a_max);
                                                        let mut light_brdf = (light.color
                                                            * light.strength
                                                            * 5.0
                                                            * clamp(
                                                                ggx(
                                                                    nl, ray.d, l, roughness,
                                                                    metallic,
                                                                ),
                                                                0.0,
                                                                1.0,
                                                            )
                                                            * omega)
                                                            / f32::pi();

                                                        light_brdf *= mask * color;

                                                        let mut found_light = false;
                                                        for light in lights.iter_mut() {
                                                            if light.pos == *light_grid {
                                                                light.brdf = lerp(
                                                                    light.brdf,
                                                                    light_brdf,
                                                                    1.0 / (sample as f32 + 1.0),
                                                                );
                                                                found_light = true;
                                                                break;
                                                            }
                                                        }

                                                        if !found_light {
                                                            lights.push(PreRenderedLight {
                                                                pos: *light_grid,
                                                                brdf: light_brdf,
                                                            });
                                                        }
                                                    }
                                                }
                                            }
                                        }

                                        let xsi_1: f32 = rng.gen();
                                        let xsi_2: f32 = rng.gen();
                                        let phi = atan((alpha * sqrt(xsi_1)) / sqrt(1.0 - xsi_1));
                                        let theta = 2.0 * f32::pi() * xsi_2;
                                        let direction = angle_to_dir(nl, theta, phi);
                                        ray = Ray::new(x + n * 0.01, direction);
                                        acc += mask * hit.emissive * e; // + mask * color * brdf;

                                        mask *= color;
                                    } else {
                                        let r2: f32 = rng.gen();
                                        let d = jitter(
                                            nl,
                                            2. * f32::pi() * rng.gen::<f32>(),
                                            sqrt(r2),
                                            sqrt(1. - r2),
                                        );

                                        let mut e = vec3f(0., 0., 0.);

                                        // Direct light sampling
                                        if depth == 0 {
                                            for (light_grid, light) in &level.lights {
                                                let light_dist =
                                                    length(Vec2f::from(*light_grid - *tile));

                                                if light_dist < light.max_distance {
                                                    let light_pos = vec3f(
                                                        light_grid.x as f32 + 0.5,
                                                        0.5,
                                                        light_grid.y as f32 + 0.5,
                                                    );
                                                    let l0 = light_pos - x;

                                                    let lr = light.max_distance;

                                                    let cos_a_max = sqrt(
                                                        1. - clamp(lr * lr / dot(l0, l0), 0., 1.),
                                                    );
                                                    let cosa = lerp(cos_a_max, 1., rng.gen());
                                                    let l = jitter(
                                                        l0,
                                                        2. * f32::pi() * rng.gen::<f32>(),
                                                        sqrt(1. - cosa * cosa),
                                                        cosa,
                                                    );

                                                    if self.shadow_ray(
                                                        Ray::new(x, l0),
                                                        Vec3i::from(light_pos),
                                                        light,
                                                        region,
                                                        &update,
                                                        settings,
                                                    ) {
                                                        let omega =
                                                            2.0 * f32::pi() * (1.0 - cos_a_max);

                                                        e += (light.color
                                                            * light.strength
                                                            * 5.0
                                                            * clamp(dot(l, n), 0., 1.)
                                                            * omega)
                                                            / f32::pi();
                                                    }
                                                }
                                            }
                                        }

                                        acc += mask * hit.emissive + mask * color * e;
                                        mask *= color;
                                        ray = Ray::new(x + n * 0.01, d);
                                    }
                                } else {
                                    if depth == 0 {
                                        empty_pixel = true;
                                    } else {
                                        tile_is_empty = false;
                                    }
                                    break;
                                }
                            }

                            if empty_pixel {
                                break;
                            }

                            color = acc; //lerp(color, acc, 1.0 / (sample as f32 + 1.0));
                            sky_abso = mask; //lerp(sky_abso, mask, 1.0 / (sample as f32 + 1.0));
                            distance = dist; //lerp(distance, dist, 1.0 / (sample as f32 + 1.0));
                        } else {
                            let mut state = TracerState {
                                is_refracted: false,
                                has_been_refracted: false,
                                last_ior: 1.0,
                            };

                            let mut acc = Vec3f::zero();
                            let mut abso = Vec3f::one();
                            let mut dist = 0.0;

                            for depth in 0..8 {
                                if let Some(hit) = self.prerender_pixel(
                                    ray,
                                    region,
                                    &update,
                                    settings,
                                    max_render_distance,
                                    palette,
                                    &mut rng,
                                ) {
                                    if depth == 0 {
                                        dist = hit.distance;
                                    }

                                    let mut n = hit.normal;
                                    if state.is_refracted {
                                        n = -n
                                    };

                                    // sample BSDF
                                    let mut out_dir: Vec3f = Vec3f::zero();
                                    let bsdf = sample_disney_bsdf(
                                        -ray.d,
                                        n,
                                        &hit,
                                        &mut out_dir,
                                        &mut state,
                                        &mut rng,
                                    );

                                    // add emissive part of the current material
                                    acc += hit.emissive * abso;

                                    // bsdf absorption (pdf are in bsdf.a)
                                    if bsdf.1 > 0.0 {
                                        abso *= bsdf.0 / bsdf.1;
                                    }

                                    // medium absorption
                                    if state.has_been_refracted {
                                        abso *= exp(-hit.distance
                                            * ((Vec3f::one() - hit.albedo) * hit.absorption));
                                    }

                                    ray.o = hit.hit_point;
                                    ray.d = out_dir;

                                    if state.is_refracted {
                                        ray.o += -n * 0.01;
                                    } else if state.has_been_refracted && !state.is_refracted {
                                        ray.o += -n * 0.01;
                                        state.last_ior = 1.;
                                    } else {
                                        ray.o += n * 0.01;
                                    }
                                } else {
                                    // No hit

                                    if depth == 0 {
                                        empty_pixel = true;
                                    } else {
                                        //acc += settings.daylight * abso;
                                    }
                                    break;
                                }
                            }

                            if empty_pixel {
                                break;
                            }

                            color = lerp(color, acc, 1.0 / (sample as f32 + 1.0));
                            sky_abso = lerp(sky_abso, abso, 1.0 / (sample as f32 + 1.0));
                            distance = lerp(distance, dist, 1.0 / (sample as f32 + 1.0));
                        }
                    }

                    buffer.set_pixel_vec3f(w, h, &color);
                    sky_abso_buffer.set_pixel_vec3f(w, h, &sky_abso);
                    distance_buffer.set((w, h), distance);
                    lights_buffer.set((w, h), lights);

                    // -- End
                }
            }

            let mut prerendered = prerendered_mutex.lock().unwrap();

            let mut sample = 0;
            if let Some(sampled) = prerendered.tile_samples.get(&vec2i(tile.x, tile.y)) {
                sample = *sampled as i32;
            }

            let tile_x = tile.x * region.grid_size;
            let tile_y = tile.y * region.grid_size;
            let s = 1.0 / (sample as f32 + 1.0);

            for h in 0..region.grid_size {
                for w in 0..region.grid_size {
                    // albedo
                    if let Some(existing) =
                        prerendered.albedo.at_vec3(vec2i(w + tile_x, h + tile_y))
                    {
                        if let Some(new_samp) = buffer.at_vec3(vec2i(w, h)) {
                            let p = lerp(existing, new_samp, s);
                            buffer.set_pixel_vec3f(w, h, &p);
                            prerendered
                                .albedo
                                .set_pixel_vec3f(w + tile_x, h + tile_y, &p);
                        }
                    }

                    // sky abso
                    if let Some(existing) = prerendered
                        .sky_absorption
                        .at_vec3(vec2i(w + tile_x, h + tile_y))
                    {
                        if let Some(new_samp) = sky_abso_buffer.at_vec3(vec2i(w, h)) {
                            let p = lerp(existing, new_samp, s);
                            sky_abso_buffer.set_pixel_vec3f(w, h, &p);
                            prerendered
                                .sky_absorption
                                .set_pixel_vec3f(w + tile_x, h + tile_y, &p);
                        }
                    }

                    // distance
                    if let Some(existing) = prerendered.distance.get_mut((w + tile_x, h + tile_y)) {
                        if let Some(new_samp) = distance_buffer.get_mut((w, h)) {
                            let d = lerp(*existing, *new_samp, s);
                            *existing = d;
                            *new_samp = d;
                        }
                    }

                    // lights
                    if let Some(new_samp) = lights_buffer.get_mut((w, h)) {
                        if let Some(existing) = prerendered.lights.get_mut((w + tile_x, h + tile_y))
                        {
                            for nl in new_samp {
                                for ex in existing.iter_mut() {
                                    if nl.pos == ex.pos {
                                        let e = ex.brdf;
                                        let n = nl.brdf;

                                        ex.brdf = lerp(e, n, s);
                                    }
                                }
                            }
                            lights_buffer.set((w, h), existing.clone());
                        } else {
                            prerendered
                                .lights
                                .set((w + tile_x, h + tile_y), new_samp.clone());
                        }
                    }
                }
            }

            sender
                .send(PreRenderResult::RenderedRegionTile(
                    region.id,
                    vec2i(
                        prerendered.albedo.dim().width,
                        prerendered.albedo.dim().height,
                    ),
                    vec2i(tile.x * region.grid_size, tile.y * region.grid_size),
                    buffer.clone(),
                    sky_abso_buffer.clone(),
                    distance_buffer.clone(),
                    lights_buffer.clone(),
                ))
                .unwrap();

            if !tile_is_empty {
                sample += 1;
            } else {
                sample = region.pathtracer_samples;
            }
            prerendered
                .tile_samples
                .insert(vec2i(tile.x, tile.y), sample as u16);
        });

        let _stop = self.get_time();
        //println!("render time {:?}", _stop - _start);

        false
    }

    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub fn prerender_pixel(
        &self,
        ray: Ray,
        region: &Region,
        update: &RegionUpdate,
        settings: &RegionDrawSettings,
        max_render_distance: i32,
        palette: &ThePalette,
        _rng: &mut ThreadRng,
    ) -> Option<Hit> {
        let mut hit = Hit::default();

        #[inline(always)]
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

        for _ii in 0..max_render_distance {
            key = Vec3i::from(i);

            if key.y < -1 {
                break;
            }

            let mut has_hit = false;

            if let Some(geo_ids) = region.geometry_areas.get(&key) {
                hit.key = Vec3f::from(key);
                for geo_id in geo_ids {
                    let mut h = Hit::default();
                    if let Some(geo_obj) = region.geometry.get(geo_id) {
                        let material = self.materials.get(&geo_obj.material_id);
                        let lro = ray.at(dist);

                        let r = Ray::new(lro, ray.d);
                        let mut t = 0.01;

                        for _ in 0..20 {
                            // Max distance a ray can travel in a unit cube
                            if t > 1.732 {
                                break;
                            }

                            let p = r.at(t);

                            let d; // = (f32::INFINITY, 0);
                            if let Some(material) = material {
                                d = material.get_distance_3d(&settings.time, p, &mut h, geo_obj);
                            } else {
                                d = MaterialFXObject::default().get_distance_3d(
                                    &settings.time,
                                    p,
                                    &mut h,
                                    geo_obj,
                                );
                            }

                            if d.0.abs() < 0.001 {
                                if dist + t < hit.distance {
                                    hit = h.clone();
                                    hit.hit_point = p;

                                    if let Some(material) = material {
                                        hit.normal =
                                            material.normal(&settings.time, p, &mut h, geo_obj);
                                    } else {
                                        hit.normal = MaterialFXObject::default().normal(
                                            &settings.time,
                                            p,
                                            &mut h,
                                            geo_obj,
                                        );
                                    }

                                    hit.distance = dist + t;
                                    hit.albedo = vec3f(0.5, 0.5, 0.5);

                                    if h.extrusion == GeoFXNodeExtrusion::None {
                                        hit.value = 1.0;
                                        geo_obj.nodes[d.1].distance_3d(
                                            &settings.time,
                                            p,
                                            &mut Some(&mut hit),
                                        );
                                    }

                                    if let Some(material) = material {
                                        hit.uv = self.get_uv_face(hit.normal, hit.hit_point).0;
                                        material.compute(&mut hit, palette, &self.textures);
                                    }
                                }

                                has_hit = true;
                            }
                            t += d.0;
                        }
                    }
                }
            }
            // Test against world tiles
            if let Some(tile) = self.tiles.get((key.x, key.y, key.z)) {
                if dist > hit.distance {
                    continue;
                }

                let mut uv = self.get_uv_face(normal, ray.at(dist)).0;
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
                            hit.albedo = vec3f(p.x, p.y, p.z);
                            hit.normal = -normal;
                            hit.distance = dist;
                            hit.hit_point = ray.at(dist);
                            has_hit = true;
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
                            if t >= 0.0 {
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
                                            hit.albedo = vec3f(col.x, col.y, col.z);
                                            hit.distance = t;
                                            hit.normal = -normal;
                                            hit.hit_point = ray.at(t);
                                            has_hit = true;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if has_hit {
                    break;
                }
            }

            let plain = (1.0 + srd - 2.0 * (ro - i)) * rdi;
            dist = min(plain.x, min(plain.y, plain.z));
            normal = equal(dist, plain) * srd;
            i += normal;
        }

        // Light Sampling
        //
        /*
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
                for (light_grid, light) in &level.lights {
                    let light_pos =
                        vec3f(light_grid.x as f32 + 0.5, 0.8, light_grid.y as f32 + 0.5);
                    let mut light_strength = light.strength;

                    if light.color_type == 1 {
                        light_strength = daylight.x;
                    }

                    let mut ro = ray.at(dist);

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

                    let light_dir = light_pos - ro;
                    let light_dist = length(light_dir);
                    if light_dist < light.max_distance {
                        ro += light_dir * 0.001;

                        let light_ray = Ray::new(ro, light_dir);

                        /*
                        let intensity = 1.0 - (light_dist / light.max_distance).clamp(0.0, 1.0);
                        //intensity *= if s == 0 { 2.0 } else { 1.0 };
                        let mut light_color =
                            Vec3f::from(intensity * light_strength / light.samples as f32);
                        if light.color_type == 0 {
                            light_color *= light.color
                        }
                        total_light += light_color;
                        */

                        if self.shadow_ray(
                            light_ray,
                            Vec3i::from(light_pos),
                            light,
                            region,
                            update,
                            settings,
                        ) {
                            let c = if settings.pbr {
                                let mut light_color = Vec3f::from(1.5 * light_strength);
                                if light.color_type == 0 {
                                    light_color *= light.color
                                }
                                let roughness = hit_props.roughness;
                                let metallic = hit_props.metallic;
                                let reflectance = hit_props.reflectance;
                                let base_color = vec3f(color.x, color.y, color.z);

                                let f0 = 0.16 * reflectance * reflectance * (1.0 - metallic)
                                    + base_color * metallic;

                                self.compute_pbr_lighting(
                                    light_pos,
                                    light_color,
                                    ro,
                                    abs(normal),
                                    -rd,
                                    base_color,
                                    roughness,
                                    f0,
                                )
                            } else {
                                let mut light_color = Vec3f::from(light_strength);
                                if light.color_type == 0 {
                                    light_color *= light.color
                                }
                                let intensity =
                                    1.0 - (light_dist / light.max_distance).clamp(0.0, 1.0);

                                light_color * intensity
                            };

                            total_light += c;
                        }
                    }
                }

                // color = color * daylight + vec4f(total_light.x, total_light.y, total_light.z, 1.0);

                let min_color = vec4f(
                    color.x * daylight.x,
                    color.y * daylight.y,
                    color.z * daylight.z,
                    1.0,
                );

                color = clamp(
                    color * daylight
                        + color * vec4f(total_light.x, total_light.y, total_light.z, 1.0),
                    min_color,
                    color,
                );
            }
            }*/

        // if let Some(saturation) = saturation {
        //     let mut hsl = TheColor::from_vec4f(color).as_hsl();
        //     hsl.y *= saturation;
        //     color = TheColor::from_hsl(hsl.x * 360.0, hsl.y.clamp(0.0, 1.0), hsl.z).to_vec4f();
        // }

        //TheColor::from_vec4f(color).to_u8_array()

        if hit.distance < f32::MAX {
            Some(hit)
        } else {
            None
        }
    }

    /// RENDERED

    #[allow(clippy::too_many_arguments)]
    pub fn rendered(
        &mut self,
        buffer: &mut TheRGBABuffer,
        region: &Region,
        update: &mut RegionUpdate,
        settings: &mut RegionDrawSettings,
        compute_delta: bool,
        palette: &ThePalette,
    ) {
        let _start = self.get_time();

        let width = buffer.dim().width as usize;
        let height = buffer.dim().height as usize;

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

        settings.pbr = false;
        if let Some(v) = region.regionfx.get(
            str!("Renderer"),
            str!("Shading"),
            &settings.time,
            TheInterpolation::Linear,
        ) {
            if let Some(value) = v.to_i32() {
                if value == 1 {
                    settings.pbr = true;
                }
            }
        }

        let mut max_render_distance = 20;
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
        //let prerender_camera = Camera::prerender(ro, rd, vec2f(width_f, height_f), fov);
        let camera = Camera::new(ro, rd, fov);

        let ppt = region.grid_size as f32;

        let mut start_x = 0;
        let mut start_y = 0;

        // Find the location in the prerendered map

        let ray = if camera_type == CameraType::TiltedIso {
            camera.create_tilted_isometric_ray2(
                vec2f(0.0, 1.0),
                vec2f(width_f, height_f),
                vec2f(width_f / ppt, height_f / ppt),
                vec2f(1.0, 1.0),
                tilted_iso_alignment,
            )
        } else {
            camera.create_ortho_ray2(
                vec2f(0.0, 1.0),
                vec2f(width_f, height_f),
                vec2f(width_f / ppt, height_f / ppt),
                vec2f(1.0, 1.0),
            )
        };

        let plane_normal = vec3f(0.0, 1.0, 0.0);
        let denom = dot(plane_normal, ray.d);

        if denom.abs() > 0.0001 {
            let t = dot(vec3f(0.0, 0.0, 0.0) - ray.o, plane_normal) / denom;
            if t >= 0.0 {
                let p = ray.o + ray.d * t;
                if let Some(data) = region.prerendered.tree.nearest_neighbor(&[p.x, p.z]) {
                    start_x = data.pixel_location.0;
                    start_y = data.pixel_location.1;

                    // if let Some(tile_coords) = &region.prerendered.tile_coords {
                    //     fn bilinear_interpolate(corners: &[Vec2f; 4], u: f32, v: f32) -> Vec2f {
                    //         Vec2f {
                    //             x: corners[0].x * (1.0 - u) * (1.0 - v)
                    //                 + corners[1].x * u * (1.0 - v)
                    //                 + corners[2].x * (1.0 - u) * v
                    //                 + corners[3].x * u * v,
                    //             y: corners[0].y * (1.0 - u) * (1.0 - v)
                    //                 + corners[1].y * u * (1.0 - v)
                    //                 + corners[2].y * (1.0 - u) * v
                    //                 + corners[3].y * u * v,
                    //         }
                    //     }
                    //     let size_x = width_f / (region.width as f32 * region.grid_size as f32);
                    //     let size_y = height_f / (region.height as f32 * region.grid_size as f32);

                    //     let x = p.x / size_x;
                    //     let y = p.y / size_y;

                    //     let uv = bilinear_interpolate(tile_coords, x, y) * vec2f(size_x, size_y);

                    //     println!("{} {}", start_x, uv.x);
                    // } else {
                    //     println!("No coords!");
                    // }
                }
            }
        }

        // TEMP TESTING

        // Render loop

        pixels
            .par_rchunks_exact_mut(width * 4)
            .enumerate()
            .for_each(|(j, line)| {
                for (i, pixel) in line.chunks_exact_mut(4).enumerate() {
                    let i = j * width + i;

                    let xx = (i % width) as f32;
                    let yy = (i / width) as f32;

                    let mut ray = if camera_type == CameraType::TiltedIso {
                        camera.create_tilted_isometric_ray2(
                            vec2f(xx / width_f, yy / height_f),
                            vec2f(width_f, height_f),
                            vec2f(width_f / ppt, height_f / ppt),
                            vec2f(1.0, 1.0),
                            tilted_iso_alignment,
                        )
                    } else if camera_mode == CameraMode::Pinhole {
                        camera.create_ray(
                            vec2f(xx / width_f, yy / height_f),
                            vec2f(width_f, height_f),
                            vec2f(1.0, 1.0),
                        )
                    } else {
                        camera.create_ortho_ray2(
                            vec2f(xx / width_f, yy / height_f),
                            vec2f(width_f, height_f),
                            vec2f(width_f / ppt, height_f / ppt),
                            vec2f(1.0, 1.0),
                        )
                    };

                    // In top down view, intersect ray with plane at 1.1 y
                    // to speed up the ray / voxel casting
                    if camera_type != CameraType::FirstPerson {
                        let plane_normal = vec3f(0.0, 1.0, 0.0);
                        let denom = dot(plane_normal, ray.d);

                        if denom.abs() > 0.0001 {
                            let t = dot(vec3f(0.0, 1.1, 0.0) - ray.o, plane_normal) / denom;
                            if t >= 0.0 {
                                ray.o += ray.d * t;
                            }
                        }
                    }

                    // --
                    //

                    let pos = vec2i(start_x + xx as i32, start_y + (height_f - yy) as i32);

                    pixel.copy_from_slice(&self.rendered_pixel(
                        ray,
                        pos,
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
        // println!("render time {:?}", _stop - _start);
    }

    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub fn rendered_pixel(
        &self,
        ray: Ray,
        pos: Vec2i,
        region: &Region,
        update: &RegionUpdate,
        settings: &RegionDrawSettings,
        camera_type: CameraType,
        level: &Level,
        saturation: &Option<f32>,
        _max_render_distance: i32,
        _palette: &ThePalette,
    ) -> RGBA {
        let mut color = vec3f(0.0, 0.0, 0.0);
        let hit_props = Hit::default();

        let rd = ray.d;
        let mut dist = 0.0;
        let normal = Vec3f::zero();
        let mut hit = false;

        if let Some(c) = region.prerendered.albedo.at_vec3(pos) {
            color = c;

            if let Some(abso) = region.prerendered.sky_absorption.at_vec3(pos) {
                // color.x += powf(settings.daylight.x * abso.x, 1.0 / 2.2);
                // color.y += powf(settings.daylight.y * abso.y, 1.0 / 2.2);
                // color.z += powf(settings.daylight.z * abso.z, 1.0 / 2.2);

                color.x += settings.daylight.x * abso.x;
                color.y += settings.daylight.y * abso.y;
                color.z += settings.daylight.z * abso.z;

                if let Some(d) = region.prerendered.distance.get((pos.x, pos.y)) {
                    dist = *d;
                }

                if let Some(lights) = region.prerendered.lights.get((pos.x, pos.y)) {
                    for light in lights.iter() {
                        fn hash_u32(seed: u32) -> u32 {
                            let mut state = seed;
                            state = (state ^ 61) ^ (state >> 16);
                            state = state.wrapping_add(state << 3);
                            state ^= state >> 4;
                            state = state.wrapping_mul(0x27d4eb2d);
                            state ^= state >> 15;
                            state
                        }

                        fn flicker_value(anim_counter: u32, intensity: f32) -> f32 {
                            let hash = hash_u32(anim_counter);
                            let flicker_value = (hash as f32 / u32::MAX as f32).clamp(0.0, 1.0);

                            let flicker_range = intensity * (flicker_value - 0.5) * 2.0;
                            (1.0 + flicker_range).clamp(0.0, 1.0)
                        }

                        let l = light.brdf * flicker_value(settings.anim_counter as u32, 0.2);
                        color += l;
                    }
                }
            }

            hit = true;
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

                            let x = (u * data.buffer[index].dim().width as f32) as i32;
                            let y = ((1.0 - v) * data.buffer[index].dim().height as f32) as i32;
                            if let Some(c) = data.buffer[index].at(vec2i(x, y)) {
                                if c[3] == 255 {
                                    let col = TheColor::from_u8_array(c).to_vec3f();
                                    color = col * settings.daylight;
                                    dist = t;
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
        if false {
            //hit {
            if level.lights.is_empty() {
                color *= settings.daylight;
            } else {
                // Sample the lights
                let mut total_light = Vec3f::new(0.0, 0.0, 0.0);
                for (light_grid, light) in &level.lights {
                    let light_pos =
                        vec3f(light_grid.x as f32 + 0.5, 0.8, light_grid.y as f32 + 0.5);
                    let mut light_strength = light.strength;

                    if light.color_type == 1 {
                        light_strength = settings.daylight.x;
                    }

                    let mut ro = ray.at(dist);

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

                    let light_dir = light_pos - ro;
                    let light_dist = length(light_dir);
                    if light_dist < light.max_distance {
                        ro += light_dir * 0.01;

                        let light_ray = Ray::new(ro, light_dir);

                        /*
                        let intensity = 1.0 - (light_dist / light.max_distance).clamp(0.0, 1.0);
                        //intensity *= if s == 0 { 2.0 } else { 1.0 };
                        let mut light_color =
                            Vec3f::from(intensity * light_strength / light.samples as f32);
                        if light.color_type == 0 {
                            light_color *= light.color
                        }
                        total_light += light_color;
                        */

                        if self.shadow_ray(
                            light_ray,
                            Vec3i::from(light_pos),
                            light,
                            region,
                            update,
                            settings,
                        ) {
                            let c = if settings.pbr {
                                let mut light_color = Vec3f::from(1.5 * light_strength);
                                if light.color_type == 0 {
                                    light_color *= light.color
                                }
                                let roughness = hit_props.roughness;
                                let metallic = hit_props.metallic;
                                let reflectance = hit_props.reflectance;
                                let base_color = vec3f(color.x, color.y, color.z);

                                let f0 = 0.16 * reflectance * reflectance * (1.0 - metallic)
                                    + base_color * metallic;

                                self.compute_pbr_lighting(
                                    light_pos,
                                    light_color,
                                    ro,
                                    abs(normal),
                                    -rd,
                                    base_color,
                                    roughness,
                                    f0,
                                )
                            } else {
                                let mut light_color = Vec3f::from(light_strength);
                                if light.color_type == 0 {
                                    light_color *= light.color
                                }
                                let intensity =
                                    1.0 - (light_dist / light.max_distance).clamp(0.0, 1.0);

                                light_color * intensity
                            };

                            total_light += c;
                        }
                    }
                }

                // color = color * daylight + vec4f(total_light.x, total_light.y, total_light.z, 1.0);

                let min_color = color;
                color += color * vec3f(total_light.x, total_light.y, total_light.z);
                color = clamp(color, min_color, color);
                /*
                let min_color = vec3f(
                    color.x * settings.daylight.x,
                    color.y * settings.daylight.y,
                    color.z * settings.daylight.z,
                );

                color = clamp(
                    color // * settings.daylight
                    + color * vec3f(total_light.x, total_light.y, total_light.z),
                    min_color,
                    color,
                    );*/
            }
        }

        if let Some(saturation) = saturation {
            let mut hsl = TheColor::from_vec3f(color).as_hsl();
            hsl.y *= saturation;
            color = TheColor::from_hsl(hsl.x * 360.0, hsl.y.clamp(0.0, 1.0), hsl.z).to_vec3f();
        }

        // Show hover
        if let Some(hover) = self.hover_pos {
            let plane_normal = vec3f(0.0, 1.0, 0.0);
            let denom = dot(plane_normal, ray.d);

            if denom.abs() > 0.0001 {
                let t = dot(vec3f(0.0, 0.0, 0.0) - ray.o, plane_normal) / denom;
                if t >= 0.0 {
                    let hp = Vec3i::from(ray.at(t));
                    if hp == hover {
                        color = TheColor::from_vec3f(color)
                            .mix(&TheColor::white(), 0.5)
                            .to_vec3f();
                    }
                }
            }
        }

        TheColor::from_vec3f(color).to_u8_array()
    }

    /// RENDER

    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        buffer: &mut TheRGBABuffer,
        region: &Region,
        update: &mut RegionUpdate,
        settings: &mut RegionDrawSettings,
        compute_delta: bool,
        palette: &ThePalette,
    ) {
        let _start = self.get_time();

        let width = buffer.dim().width as usize;
        let height = buffer.dim().height as usize;

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

        settings.pbr = false;
        if let Some(v) = region.regionfx.get(
            str!("Renderer"),
            str!("Shading"),
            &settings.time,
            TheInterpolation::Linear,
        ) {
            if let Some(value) = v.to_i32() {
                if value == 1 {
                    settings.pbr = true;
                }
            }
        }

        let mut max_render_distance = 20;
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
                            let t = dot(vec3f(0.0, 1.1, 0.0) - ray.o, plane_normal) / denom;
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
        _palette: &ThePalette,
    ) -> RGBA {
        let mut color = vec4f(0.0, 0.0, 0.0, 1.0);
        let mut hit_props = Hit::default();

        #[inline(always)]
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

            // First person is limited to 1 y
            if camera_type == CameraType::FirstPerson && key.y > 1 {
                break;
            }

            if let Some(model) = region.models.get(&(key.x, key.y, key.z)) {
                let mut lro = ray.at(dist);
                lro -= Vec3f::from(key);

                let mut wallfx_offset = Vec3i::zero();
                let mut alpha = 1.0;

                if let Some(wallfx) = update.wallfx.get(&(key.x, key.z)) {
                    let mut valid = true;
                    let mut xx = 0;
                    let mut yy = 0;
                    let d =
                        (update.server_tick - wallfx.at_tick) as f32 + settings.delta_in_tick - 1.0;
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
                        wallfx_offset.x -= xx;
                        wallfx_offset.y -= yy;
                    } else {
                        //uv = vec2f(-1.0, -1.0);
                        wallfx_offset = vec3i(region.grid_size, region.grid_size, region.grid_size);
                    }
                }

                if let Some(hit_struct) = model.dda(&Ray::new(lro, ray.d), wallfx_offset) {
                    hit = true;
                    color = hit_struct.color;
                    // color = vec4f(
                    //     hit_struct.normal.x,
                    //     hit_struct.normal.y,
                    //     hit_struct.normal.z,
                    //     1.0,
                    // );
                    dist += hit_struct.distance;
                    normal = hit_struct.normal;
                    hit_props = hit_struct.clone();
                    break;
                }
                /*
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
                    }*/
            }
            // Test against world tiles
            else if let Some(tile) = self.tiles.get((key.x, key.y, key.z)) {
                let mut uv = self.get_uv_face(normal, ray.at(dist)).0;
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
                                    let y =
                                        ((1.0 - v) * data.buffer[index].dim().height as f32) as i32;
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
                for (light_grid, light) in &level.lights {
                    let light_pos =
                        vec3f(light_grid.x as f32 + 0.5, 0.8, light_grid.y as f32 + 0.5);
                    let mut light_strength = light.strength;

                    if light.color_type == 1 {
                        light_strength = daylight.x;
                    }

                    let mut ro = ray.at(dist);

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

                    let light_dir = light_pos - ro;
                    let light_dist = length(light_dir);
                    if light_dist < light.max_distance {
                        ro += light_dir * 0.001;

                        let light_ray = Ray::new(ro, light_dir);

                        /*
                        let intensity = 1.0 - (light_dist / light.max_distance).clamp(0.0, 1.0);
                        //intensity *= if s == 0 { 2.0 } else { 1.0 };
                        let mut light_color =
                            Vec3f::from(intensity * light_strength / light.samples as f32);
                        if light.color_type == 0 {
                            light_color *= light.color
                        }
                        total_light += light_color;
                        */

                        if self.shadow_ray(
                            light_ray,
                            Vec3i::from(light_pos),
                            light,
                            region,
                            update,
                            settings,
                        ) {
                            let c = if settings.pbr {
                                let mut light_color = Vec3f::from(1.5 * light_strength);
                                if light.color_type == 0 {
                                    light_color *= light.color
                                }
                                let roughness = hit_props.roughness;
                                let metallic = hit_props.metallic;
                                let reflectance = hit_props.reflectance;
                                let base_color = vec3f(color.x, color.y, color.z);

                                let f0 = 0.16 * reflectance * reflectance * (1.0 - metallic)
                                    + base_color * metallic;

                                self.compute_pbr_lighting(
                                    light_pos,
                                    light_color,
                                    ro,
                                    abs(normal),
                                    -rd,
                                    base_color,
                                    roughness,
                                    f0,
                                )
                            } else {
                                let mut light_color = Vec3f::from(light_strength);
                                if light.color_type == 0 {
                                    light_color *= light.color
                                }
                                let intensity =
                                    1.0 - (light_dist / light.max_distance).clamp(0.0, 1.0);

                                light_color * intensity
                            };

                            total_light += c;
                        }
                    }
                }

                // color = color * daylight + vec4f(total_light.x, total_light.y, total_light.z, 1.0);

                let min_color = vec4f(
                    color.x * daylight.x,
                    color.y * daylight.y,
                    color.z * daylight.z,
                    1.0,
                );

                color = clamp(
                    color * daylight
                        + color * vec4f(total_light.x, total_light.y, total_light.z, 1.0),
                    min_color,
                    color,
                );
            }
        }

        if let Some(saturation) = saturation {
            let mut hsl = TheColor::from_vec4f(color).as_hsl();
            hsl.y *= saturation;
            color = TheColor::from_hsl(hsl.x * 360.0, hsl.y.clamp(0.0, 1.0), hsl.z).to_vec4f();
        }

        // Show hover
        if let Some(hover) = self.hover_pos {
            let plane_normal = vec3f(0.0, 1.0, 0.0);
            let denom = dot(plane_normal, ray.d);

            if denom.abs() > 0.0001 {
                let t = dot(vec3f(0.0, 0.0, 0.0) - ray.o, plane_normal) / denom;
                if t >= 0.0 {
                    let hp = Vec3i::from(ray.at(t));
                    if hp == hover {
                        color = TheColor::from_vec4f(color)
                            .mix(&TheColor::white(), 0.5)
                            .to_vec4f();
                    }
                }
            }
        }

        TheColor::from_vec4f(color).to_u8_array()
    }

    #[inline(always)]
    pub fn shadow_ray(
        &self,
        ray: Ray,
        light_pos: Vec3i,
        light: &Light,
        _region: &Region,
        _update: &RegionUpdate,
        _settings: &RegionDrawSettings,
    ) -> bool {
        #[inline(always)]
        fn equal(l: f32, r: Vec3f) -> Vec3f {
            vec3f(
                if l == r.x { 1.0 } else { 0.0 },
                if l == r.y { 1.0 } else { 0.0 },
                if l == r.z { 1.0 } else { 0.0 },
            )
        }

        let ro = ray.o;
        let rd = ray.d;

        if Vec3i::from(ro) == light_pos {
            return true;
        }

        let mut i = floor(ro);
        let mut dist; // = 0.0;

        let mut normal;
        let srd = signum(rd);

        let rdi = 1.0 / (2.0 * rd);

        let mut key: Vec3<i32>;

        for _ in 0..light.max_distance as i32 {
            key = Vec3i::from(i);

            if key == light_pos {
                return true;
            }

            if key.y < -1 {
                return false;
            }

            if key.y > 1 {
                return false;
            }

            /*
            if let Some(model) = region.models.get(&(key.x, key.y, key.z)) {
                let mut lro = ray.at(dist);
                lro -= Vec3f::from(key);

                let mut wallfx_offset = Vec3i::zero();
                let mut alpha = 1.0;

                if let Some(wallfx) = update.wallfx.get(&(key.x, key.z)) {
                    let mut valid = true;
                    let mut xx = 0;
                    let mut yy = 0;
                    let d =
                        (update.server_tick - wallfx.at_tick) as f32 + settings.delta_in_tick - 1.0;
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
                        wallfx_offset.x -= xx;
                        wallfx_offset.y -= yy;
                    } else {
                        //uv = vec2f(-1.0, -1.0);
                        wallfx_offset = vec3i(region.grid_size, region.grid_size, region.grid_size);
                    }
                }

                if let Some(_hit_struct) = model.dda(&Ray::new(lro, ray.d), wallfx_offset) {
                    return false;
                }
                }*/
            // Test against world tiles
            if let Some(tile) = self.tiles.get((key.x, key.y, key.z)) {
                if let Some(data) = self.textures.get(tile) {
                    if data.blocking {
                        return false;
                    }
                }
            }

            let plain = (1.0 + srd - 2.0 * (ro - i)) * rdi;
            dist = min(plain.x, min(plain.y, plain.z));
            normal = equal(dist, plain) * srd;
            i += normal;
        }

        false
    }

    #[inline(always)]
    pub fn get_uv_face(&self, normal: Vec3f, hp: Vec3f) -> (Vec2f, usize) {
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
            0 => (Vec2f::new(frac(hp.z), 1.0 - frac(hp.y)), 0), // X-axis face
            1 => (Vec2f::new(frac(hp.x), frac(hp.z)), 1),       // Y-axis face
            2 => (Vec2f::new(frac(hp.x), 1.0 - frac(hp.y)), 2), // Z-axis face
            _ => (Vec2f::zero(), 0),
        }
    }

    pub fn set_region(&mut self, region: &Region) {
        self.tiles.clear();
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

        let ppt = region.grid_size as f32;

        let camera = Camera::new(ro, rd, fov);
        let ray = if camera_type == CameraType::TiltedIso {
            camera.create_tilted_isometric_ray2(
                vec2f(
                    screen_coord.x as f32 / width_f,
                    1.0 - screen_coord.y as f32 / height_f,
                ),
                vec2f(width_f, height_f),
                vec2f(width_f / ppt, height_f / ppt),
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
            camera.create_ortho_ray2(
                vec2f(
                    screen_coord.x as f32 / width_f,
                    1.0 - screen_coord.y as f32 / height_f,
                ),
                vec2f(width_f, height_f),
                vec2f(width_f / ppt, height_f / ppt),
                vec2f(1.0, 1.0),
            )
        };

        let plane_normal = vec3f(0.0, 1.0, 0.0);
        let denom = dot(plane_normal, ray.d);

        if denom.abs() > 0.0001 {
            let t = dot(vec3f(0.0, 0.0, 0.0) - ray.o, plane_normal) / denom;
            if t >= 0.0 {
                let hit = ray.o + ray.d * t;
                //Some(Vec3i::from(hit))

                let key = Vec3i::from(hit);
                return Some(vec3i(key.x, key.y, key.z));
            }
        }
        None

        /*
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
        None*/
    }

    fn g1v(&self, dot_nv: f32, k: f32) -> f32 {
        1.0 / (dot_nv * (1.0 - k) + k)
    }

    #[allow(clippy::too_many_arguments)]
    fn compute_pbr_lighting(
        &self,
        light_pos: Vec3f,
        light_color: Vec3f,
        position: Vec3f,
        n: Vec3f,
        v: Vec3f,
        albedo: Vec3f,
        roughness: f32,
        f0: Vec3f,
    ) -> Vec3f {
        let alpha = roughness * roughness;
        let l = normalize(light_pos - position);
        let h = normalize(v + l);

        let dot_nl = clamp(dot(n, l), 0.0, 1.0);
        let dot_nv = clamp(dot(n, v), 0.0, 1.0);
        let dot_nh = clamp(dot(n, h), 0.0, 1.0);
        let dot_lh = clamp(dot(l, h), 0.0, 1.0);

        let alpha_sqr = alpha * alpha;
        let pi = std::f32::consts::PI;
        let denom = dot_nh * dot_nh * (alpha_sqr - 1.0) + 1.0;
        let d = alpha_sqr / (pi * denom * denom);

        let dot_lh_5 = (1.0_f32 - dot_lh).powf(5.0);
        let f = f0 + (1.0 - f0) * dot_lh_5;

        let k = alpha / 2.0;
        let vis = self.g1v(dot_nl, k) * self.g1v(dot_nv, k);

        let specular = d * f * vis;

        let inv_pi = std::f32::consts::FRAC_1_PI;
        let diffuse = albedo * inv_pi;

        (diffuse + specular) * light_color * dot_nl
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
