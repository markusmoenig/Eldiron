use crate::SampleMode;
use crate::{
    AccumBuffer, Assets, Batch3D, Chunk, D3Camera, HitInfo, MaterialRole, Pixel, PixelSource, Ray,
    Scene, ShapeFXGraph, pixel_to_vec4,
};
use SampleMode::*;
use bvh::aabb::Aabb;
use bvh::aabb::Bounded;
use bvh::ray::Ray as BvhRay;
use rand::Rng;
use rayon::prelude::*;
use vek::{Vec2, Vec3, Vec4};

fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn _aces_tonemap(x: f32) -> f32 {
    const A: f32 = 2.51;
    const B: f32 = 0.03;
    const C: f32 = 2.43;
    const D: f32 = 0.59;
    const E: f32 = 0.14;
    ((x * (A * x + B)) / (x * (C * x + D) + E)).clamp(0.0, 1.0)
}

pub struct Tracer {
    /// SampleMode, default is Nearest.
    pub sample_mode: SampleMode,

    /// Background color (Sky etc.)
    pub background_color: Option<[u8; 4]>,

    /// Hash for animation
    pub hash_anim: u32,

    /// Optional per-batch bounding boxes for fast culling
    pub static_bboxes: Vec<Aabb<f32, 3>>,
    pub dynamic_bboxes: Vec<Aabb<f32, 3>>,

    /// The rendergraph
    pub render_graph: ShapeFXGraph,
    render_hit: Vec<u16>,
    render_miss: Vec<u16>,

    pub hour: f32,
}

impl Default for Tracer {
    fn default() -> Self {
        Tracer::new()
    }
}

impl Tracer {
    pub fn new() -> Self {
        Self {
            sample_mode: Nearest,
            background_color: None,
            static_bboxes: vec![],
            dynamic_bboxes: vec![],
            hash_anim: 0,

            render_graph: ShapeFXGraph::default(),
            render_hit: vec![],
            render_miss: vec![],
            hour: 12.0,
        }
    }

    /// Sets the sample mode using the builder pattern.
    pub fn sample_mode(mut self, sample_mode: SampleMode) -> Self {
        self.sample_mode = sample_mode;
        self
    }

    /// Sets the background using the builder pattern.
    pub fn background(mut self, background: Pixel) -> Self {
        self.background_color = Some(background);
        self
    }

    /// Precomputes the bounding boxes of all static batches.
    pub fn compute_static_bboxes(&mut self, scene: &Scene) {
        self.static_bboxes.clear();
        for batch in &scene.d3_static {
            self.static_bboxes.push(batch.aabb());
        }
    }

    /// Precomputes the bounding boxes of all dynamic batches.
    pub fn compute_dynamic_bboxes(&mut self, scene: &Scene) {
        self.dynamic_bboxes.clear();
        for batch in &scene.d3_dynamic {
            self.dynamic_bboxes.push(batch.aabb());
        }
    }

    /// Path trace the scene.
    #[allow(clippy::too_many_arguments)]
    pub fn trace(
        &mut self,
        camera: &dyn D3Camera,
        scene: &mut Scene,
        buffer: &mut AccumBuffer,
        tile_size: usize,
        assets: &Assets,
    ) {
        let width = buffer.width;
        let height = buffer.height;
        let frame = buffer.frame;

        /// Generate a hash value for the given animation frame.
        /// We use it for random light flickering.
        fn hash_u32(seed: u32) -> u32 {
            let mut state = seed;
            state = (state ^ 61) ^ (state >> 16);
            state = state.wrapping_add(state << 3);
            state ^= state >> 4;
            state = state.wrapping_mul(0x27d4eb2d);
            state ^= state >> 15;
            state
        }
        self.hash_anim = hash_u32(scene.animation_frame as u32);

        self.compute_static_bboxes(scene);
        self.compute_dynamic_bboxes(scene);

        self.render_hit = self.render_graph.collect_nodes_from(0, 0);
        self.render_miss = self.render_graph.collect_nodes_from(0, 1);

        // Precompute hit node values
        for node in &mut self.render_hit {
            self.render_graph.nodes[*node as usize].render_setup(self.hour);
        }

        // Precompute missed node values
        for node in &mut self.render_miss {
            self.render_graph.nodes[*node as usize].render_setup(self.hour);
        }

        // Divide the screen into tiles
        let mut tiles = Vec::new();
        for y in (0..height).step_by(tile_size) {
            for x in (0..width).step_by(tile_size) {
                tiles.push(TileRect {
                    x,
                    y,
                    width: tile_size.min(width - x),
                    height: tile_size.min(height - y),
                });
            }
        }

        let screen_size = Vec2::new(width as f32, height as f32);

        // Parallel process each tile
        let tile_results: Vec<(TileRect, Vec<Vec4<f32>>)> = tiles
            .par_iter()
            .map(|tile| {
                let tile = *tile;
                let mut lin_tile = vec![Vec4::zero(); tile.width * tile.height];
                let mut rng = rand::rng();

                for ty in 0..tile.height {
                    for tx in 0..tile.width {
                        let mut ret: Vec3<f32> = Vec3::zero();
                        let mut throughput: Vec3<f32> = Vec3::one();

                        let screen_uv = Vec2::new(
                            (tile.x + tx) as f32 / screen_size.x,
                            1.0 - (tile.y + ty) as f32 / screen_size.y,
                        );

                        let jitter = Vec2::new(rng.random::<f32>(), rng.random::<f32>());
                        let mut ray = camera.create_ray(screen_uv, screen_size, jitter);
                        let mut bvh_ray = BvhRay::new(
                            nalgebra::Point3::new(ray.origin.x, ray.origin.y, ray.origin.z),
                            nalgebra::Vector3::new(ray.dir.x, ray.dir.y, ray.dir.z),
                        );
                        let camera_pos = ray.origin;

                        let bounces = 8;
                        for _ in 0..bounces {
                            let mut hitinfo = HitInfo::default();

                            // Evaluate chunks
                            for (_coord, chunk) in scene.chunks.iter() {
                                // if let Some(bbox) = self.static_bboxes.get(i) {
                                //     if !bvh_ray.intersects_aabb(bbox) {
                                //         continue;
                                //     }
                                // }

                                for batch in &chunk.batches3d {
                                    if let Some(mut hit) = batch.intersect(&ray, false) {
                                        if hit.t < hitinfo.t
                                            && self.evaluate_hit(
                                                &ray,
                                                scene,
                                                batch,
                                                &mut hit,
                                                assets,
                                                Some(chunk),
                                            )
                                        {
                                            hitinfo = hit;
                                        }
                                    }
                                }

                                if let Some(batch) = &chunk.terrain_batch3d {
                                    if let Some(mut hit) = batch.intersect(&ray, false) {
                                        if hit.t < hitinfo.t
                                            && self.evaluate_hit(
                                                &ray,
                                                scene,
                                                batch,
                                                &mut hit,
                                                assets,
                                                Some(chunk),
                                            )
                                        {
                                            hitinfo = hit;
                                        }
                                    }
                                }
                            }

                            // Evaluate static
                            for (i, batch) in scene.d3_static.iter().enumerate() {
                                if let Some(bbox) = self.static_bboxes.get(i) {
                                    if !bvh_ray.intersects_aabb(bbox) {
                                        continue;
                                    }
                                }

                                if let Some(mut hit) = batch.intersect(&ray, false) {
                                    if hit.t < hitinfo.t
                                        && self.evaluate_hit(
                                            &ray, scene, batch, &mut hit, assets, None,
                                        )
                                    {
                                        hitinfo = hit;
                                    }
                                }
                            }

                            // Evaluate dynamic
                            for (i, batch) in scene.d3_dynamic.iter().enumerate() {
                                if let Some(bbox) = self.dynamic_bboxes.get(i) {
                                    if !bvh_ray.intersects_aabb(bbox) {
                                        continue;
                                    }
                                }

                                if let Some(mut hit) = batch.intersect(&ray, false) {
                                    if hit.t < hitinfo.t
                                        && self.evaluate_hit(
                                            &ray, scene, batch, &mut hit, assets, None,
                                        )
                                    {
                                        hitinfo = hit;
                                    }
                                }
                            }

                            // Hit
                            if hitinfo.t < f32::MAX {
                                if let Some(normal) = hitinfo.normal {
                                    if hitinfo.emissive != Vec3::zero() {
                                        ret += hitinfo.emissive * throughput;
                                        break;
                                    }

                                    // Direct Lighting
                                    let world = ray.at(hitinfo.t);
                                    let mut direct: Vec3<f32> = Vec3::zero();
                                    for light in scene.lights.iter().chain(&scene.dynamic_lights) {
                                        if let Some(light_color) =
                                            light.radiance_at(world, Some(normal), self.hash_anim)
                                        {
                                            direct += light_color * 10.0;
                                        }
                                    }
                                    let brdf = hitinfo.albedo / std::f32::consts::PI;
                                    ret += direct * (throughput * brdf);

                                    // New ray dir based on specular
                                    let p_spec = hitinfo.specular_weight.clamp(0.0, 1.0);
                                    let p_diff = 1.0 - p_spec;

                                    let choose_spec = rng.random::<f32>() < p_spec;
                                    let pdf = if choose_spec { p_spec } else { p_diff };

                                    if choose_spec {
                                        ray.dir = self.reflect(ray.dir, normal);
                                        throughput *= hitinfo.specular_weight / pdf;
                                    } else {
                                        ray.dir = self.sample_cosine(normal, &mut rng);
                                        throughput *= (hitinfo.albedo * p_diff)
                                            / (pdf * std::f32::consts::PI);
                                    }

                                    ray.origin = ray.at(hitinfo.t) + normal * 0.01;
                                    bvh_ray = BvhRay::new(
                                        nalgebra::Point3::new(
                                            ray.origin.x,
                                            ray.origin.y,
                                            ray.origin.z,
                                        ),
                                        nalgebra::Vector3::new(ray.dir.x, ray.dir.y, ray.dir.z),
                                    );

                                    // Russian roulete
                                    let p = throughput
                                        .x
                                        .max(throughput.y.max(throughput.z))
                                        .clamp(0.001, 1.0);
                                    if rng.random::<f32>() > p {
                                        break;
                                    }
                                    throughput *= 1.0 / p;
                                } else {
                                    println!("no normal");
                                    break;
                                }
                            } else if !self.render_miss.is_empty() {
                                // Call post-processing for missed geometry hits (sky)
                                let mut color = Vec4::new(0.0, 0.0, 0.0, 1.0);
                                for node in &self.render_miss {
                                    self.render_graph.nodes[*node as usize].render_miss_d3(
                                        &mut color,
                                        &camera_pos,
                                        &ray,
                                        &screen_uv,
                                        self.hour,
                                    );
                                }
                                let mut col = Vec3::new(color.x, color.y, color.z);
                                col = col.map(srgb_to_linear);
                                ret += col * throughput;
                                break;
                            }
                        }

                        lin_tile[ty * tile.width + tx] = Vec4::new(ret.x, ret.y, ret.z, 1.0);
                    }
                }

                (tile, lin_tile)
            })
            .collect();

        let t = 1.0 / (frame as f32 + 1.0);
        for (tile, lin_tile) in tile_results {
            for ty in 0..tile.height {
                for tx in 0..tile.width {
                    let gx = tile.x + tx;
                    let gy = tile.y + ty;

                    let old = buffer.get_pixel(gx, gy); // linear HDR
                    let new = lin_tile[ty * tile.width + tx]; // linear HDR

                    let blended = old * (1.0 - t) + new * t; // running average
                    buffer.set_pixel(gx, gy, blended);
                }
            }
        }
        buffer.frame += 1;
    }

    fn evaluate_hit(
        &self,
        ray: &Ray,
        scene: &Scene,
        batch: &Batch3D,
        hit: &mut HitInfo,
        assets: &Assets,
        chunk: Option<&Chunk>,
    ) -> bool {
        let mut texel = match batch.source {
            PixelSource::StaticTileIndex(index) => {
                let textile = &assets.tile_list[index as usize];
                let index = scene.animation_frame % textile.textures.len();

                /*
                if let Some(mut normal) = hit.normal {
                    let texel = pixel_to_vec4(&&textile.textures[index].sample_with_normal(
                        hit.uv.x,
                        hit.uv.y,
                        self.sample_mode,
                        batch.repeat_mode,
                        Some(&mut normal),
                        0.2,
                    ));
                    hit.normal = Some(normal);
                    texel
                } else {
                    */
                pixel_to_vec4(&&textile.textures[index].sample(
                    hit.uv.x,
                    hit.uv.y,
                    self.sample_mode,
                    batch.repeat_mode,
                ))
            }
            PixelSource::DynamicTileIndex(index) => {
                let textile = &scene.dynamic_textures[index as usize];
                let index = scene.animation_frame % textile.textures.len();
                pixel_to_vec4(&textile.textures[index].sample(
                    hit.uv.x,
                    hit.uv.y,
                    self.sample_mode,
                    batch.repeat_mode,
                ))
            }
            PixelSource::Pixel(col) => pixel_to_vec4(&col),
            PixelSource::Terrain => {
                // if let Some(terrain) = &scene.terrain {
                //     let w = ray.at(hit.t);
                //     pixel_to_vec4(&terrain.sample_baked(Vec2::new(w.x, w.y)))
                // } else {
                if let Some(chunk) = chunk {
                    let w = ray.at(hit.t);
                    let texel = chunk.sample_terrain_texture(Vec2::new(w.x, w.z), Vec2::one());
                    pixel_to_vec4(&texel)
                } else {
                    Vec4::zero()
                }
            }
            _ => Vec4::zero(),
        };
        let tex_lin = texel.map(srgb_to_linear);
        if let Some(material) = &batch.material {
            let value = material.modifier.modify(&texel, &material.value);
            match &material.role {
                MaterialRole::Matte => {
                    hit.specular_weight = 1.0 - value;
                }
                MaterialRole::Glossy => {
                    hit.specular_weight = value;
                }
                MaterialRole::Metallic => {
                    let m = value;
                    let inv_m = 1.0 - m;
                    texel = Vec4::new(
                        // F₀ tint
                        texel.x * inv_m + m,
                        texel.y * inv_m + m,
                        texel.z * inv_m + m,
                        texel.w,
                    );
                    hit.specular_weight = m;
                }
                MaterialRole::Emissive => {
                    hit.emissive = tex_lin.xyz() * material.value * 10.0;
                }
                _ => {}
            }
        }

        texel[3] = 1.0;

        if texel[3] == 1.0 {
            hit.albedo = Vec3::new(tex_lin.x, tex_lin.y, tex_lin.z);
            true
        } else {
            false
        }
    }

    #[inline(always)]
    pub fn reflect(&self, i: Vec3<f32>, n: Vec3<f32>) -> Vec3<f32> {
        i - 2.0 * i.dot(n) * n
    }

    #[inline(always)]
    fn _random_unit_vector<R: Rng>(&self, rng: &mut R) -> Vec3<f32> {
        let z = rng.random::<f32>() * 2.0 - 1.0;
        let a = rng.random::<f32>() * std::f32::consts::TAU;
        let r = (1.0 - z * z).sqrt();
        let x = r * a.cos();
        let y = r * a.sin();
        Vec3::new(x, y, z)
    }

    #[inline(always)]
    fn sample_cosine<R: Rng>(&self, n: Vec3<f32>, rng: &mut R) -> Vec3<f32> {
        // polar coords in local space
        let r1: f32 = rng.random();
        let r2: f32 = rng.random();
        let phi = 2.0 * std::f32::consts::PI * r1;
        let r = r2.sqrt();
        let local = Vec3::new(phi.cos() * r, phi.sin() * r, (1.0 - r2).sqrt());

        // build TBN basis
        let w = n;
        let a = if w.x.abs() > 0.1 {
            Vec3::unit_y()
        } else {
            Vec3::unit_x()
        };
        let v = w.cross(a).normalized();
        let u = v.cross(w);
        // transform to world
        (u * local.x + v * local.y + w * local.z).normalized()
    }
}

/// A rectangle struct which represents a Tile
#[derive(Clone, Copy)]
struct TileRect {
    x: usize,
    y: usize,
    width: usize,
    height: usize,
}
