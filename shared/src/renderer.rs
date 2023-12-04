use crate::prelude::*;

pub struct Renderer {
    pub tiles: FxHashMap<Uuid, TheRGBATile>,
}

#[allow(clippy::new_without_default)]
impl Renderer {
    pub fn new() -> Self {
        Self {
            tiles: FxHashMap::default(),
        }
    }

    pub fn render(
        &mut self,
        buffer: &mut TheRGBABuffer,
        dim: &TheDim,
        pos: Vec2i,
        ctx: &mut TheContext,
    ) {
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

                let camera = Camera::new(vec3f(0.5, 0.5, 2.0), vec3f(0.5, 0.5, 0.0), 70.0);
                let ray = camera.create_ray(vec2f(xx / width_f, yy / height_f), vec2f(width_f, height_f), vec2f(0.0, 0.0));

                let index = y as usize * stride * 4 + x as usize * 4;
                pixels[index..index+4].copy_from_slice(&self.render_pixel(ray));//&[(xx / width as f32 * 255.0) as u8, (yy / height as f32 * 255.0) as u8, 0, 255]);
            }
        }
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

        let mut normal;//= Vec3f::zero();
        let srd = signum(rd);

        let rdi = 1.0 / (2.0 * rd);

        let mut key: Vec3<i32>;// = Vec3i::zero();

        for _ii in 0..20 {
            key = Vec3i::from(i);

            //println!("{}", key);

            if key.x == 0 && key.y == 0 && key.z == 0 {
            // if key.y <= -1 {
                pixel = WHITE;
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

    pub fn set_region(&mut self, region: &Region) {

    }

    pub fn set_tiles(&mut self, tiles: FxHashMap<Uuid, TheRGBATile>) {
        self.tiles = tiles;
    }
    // pub fn draw_region(
    //     &mut self,
    //     buffer: &mut TheRGBABuffer,
    //     region: &Region,
    //     ctx: &mut TheContext,
    // ) {
    //     for (coord, tile) in &region.layers[0].tiles {
    //         self.draw_tile(
    //             vec2i(coord.0 as i32, coord.1 as i32),
    //             buffer,
    //             region.grid_size,
    //             *tile,
    //             ctx,
    //         );
    //     }
    // }

    // pub fn draw_tile(
    //     &mut self,
    //     at: Vec2i,
    //     buffer: &mut TheRGBABuffer,
    //     grid: i32,
    //     tile: Uuid,
    //     ctx: &mut TheContext,
    // ) {
    //     if let Some(data) = self.tiles.get(&tile) {
    //         let x = (at.x * grid) as usize;
    //         let y = (at.y * grid) as usize;
    //         let stride = buffer.stride();
    //         ctx.draw.copy_slice(
    //             buffer.pixels_mut(),
    //             data.buffer[0].pixels(),
    //             &(x, y, 24, 24),
    //             stride,
    //         );
    //     }
    // }
}
