use crate::prelude::*;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct Renderer {
    pub textures: FxHashMap<Uuid, TheRGBATile>,
    pub tiles: FxHashMap<(i32, i32, i32), Uuid>,
    pub position: Vec3f,
}

#[allow(clippy::new_without_default)]
impl Renderer {
    pub fn new() -> Self {
        Self {
            textures: FxHashMap::default(),
            tiles: FxHashMap::default(),
            position: Vec3f::zero(),
        }
    }

    pub fn render(&mut self, buffer: &mut TheRGBABuffer, dim: &TheDim, _ctx: &mut TheContext) {
        let start = self.get_time();

        let stride = buffer.stride();
        let pixels = buffer.pixels_mut();

        let width = dim.width;
        let height = dim.height;

        let width_f = dim.width as f32;
        let height_f = dim.height as f32;

        for y in 0..height {
            for x in 0..width {
                let i = y * width + x;

                let xx = (i % width) as f32;
                let yy = height_f - (i / width) as f32;

                let ro = vec3f(self.position.x + 0.5, 0.5, self.position.z + 0.5);
                let mut rd = ro;
                rd.z -= 2.0;

                let camera = Camera::new(ro, rd, 70.0);
                let ray = camera.create_ray(
                    vec2f(xx / width_f, yy / height_f),
                    vec2f(width_f, height_f),
                    vec2f(0.0, 0.0),
                );

                let index = y as usize * stride * 4 + x as usize * 4;
                pixels[index..index + 4].copy_from_slice(&self.render_pixel(ray));
                //&[(xx / width as f32 * 255.0) as u8, (yy / height as f32 * 255.0) as u8, 0, 255]);
            }
        }

        let _stop = self.get_time();
        println!("render time {:?}", _stop - start);
    }

    #[inline(always)]
    pub fn render_pixel(&self, ray: Ray) -> RGBA {
        //let mut set : FxHashSet<Vec3i> = FxHashSet::default();
        //set.insert(vec3i(0, 0, 0));

        let mut pixel = BLACK;

        // Based on https://www.shadertoy.com/view/ct33Rn

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

        let mut key: Vec3<i32>; // = Vec3i::zero();

        for _ii in 0..5 {
            key = Vec3i::from(i);

            //println!("{}", key);

            //if key.x == 0 && key.y == 0 && key.z == 0 {
            // if key.y <= -1 {
            if let Some(tile) = self.tiles.get(&(key.x, key.y, key.z)) {
                let uv = self.get_uv(normal, ray.at(dist));
                //pixel = [(uv.x * 255.0) as u8, (uv.y * 255.0) as u8, 0, 255];
                if let Some(texture) = self.textures.get(tile) {
                    if let Some(p) = texture.buffer[0].at_f(uv) {
                        pixel = p;
                    }
                }
                break;
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

        pixel
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
            1 => Vec2f::new(frac(hp.x), 1.0 - frac(hp.z)), // Y-axis face
            2 => Vec2f::new(frac(hp.x), 1.0 - frac(hp.y)), // Z-axis face
            _ => Vec2f::zero(),
        }
    }

    pub fn set_region(&mut self, region: &Region) {
        self.tiles.clear();
        for (pos, tile) in &region.tiles {
            for i in 0..3 {
                if i == 0 {
                    if let Some(tile_uuid) = tile.layers[i] {
                        self.tiles.insert((pos.0, -1, pos.1), tile_uuid);
                    }
                } else if i == 1 {
                    if let Some(tile_uuid) = tile.layers[i] {
                        self.tiles.insert((pos.0, 0, pos.1), tile_uuid);
                    }
                } else if i == 2 {
                    if let Some(tile_uuid) = tile.layers[i] {
                        self.tiles.insert((pos.0, 1, pos.1), tile_uuid);
                    }
                }
            }
        }
        println!("{:?}", self.tiles);
    }

    pub fn set_textures(&mut self, tiles: FxHashMap<Uuid, TheRGBATile>) {
        self.textures = tiles;
    }

    pub fn set_position(&mut self, position: Vec3i) {
        self.position = position.into();
    }

    /// Gets the current time in milliseconds
    fn get_time(&self) -> u128 {
        let stop = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");
        stop.as_millis()
    }
}
