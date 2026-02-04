use crate::{Pixel, Shader, vec4_to_pixel};
use vek::{Vec2, Vec4};

pub struct GridShader {
    grid_size: f32,
    subdivisions: f32,
    offset: Vec2<f32>,
}

impl Shader for GridShader {
    fn new() -> Self {
        Self {
            grid_size: 30.0,
            subdivisions: 2.0,
            offset: Vec2::zero(),
        }
    }

    fn set_parameter_f32(&mut self, key: &str, value: f32) {
        #[allow(clippy::single_match)]
        match key {
            "grid_size" => self.grid_size = value,
            "subdivisions" => self.subdivisions = value,
            _ => {}
        }
    }

    fn set_parameter_vec2(&mut self, key: &str, value: Vec2<f32>) {
        #[allow(clippy::single_match)]
        match key {
            "offset" => self.offset = value,
            _ => {}
        }
    }

    fn shade_pixel(&self, uv: Vec2<f32>, screen: Vec2<f32>) -> Pixel {
        fn odd(n: i32) -> bool {
            n % 2 != 0
        }

        fn closest_mul(delta: Vec2<f32>, value: Vec2<f32>) -> Vec2<f32> {
            delta * (value / delta).map(|v| v.round())
        }

        fn mul_dist(delta: Vec2<f32>, value: Vec2<f32>) -> Vec2<f32> {
            (value - closest_mul(delta, value)).map(|v| v.abs())
        }

        fn align_pixel(point: Vec2<f32>, thickness: i32) -> Vec2<f32> {
            if odd(thickness) {
                (point - Vec2::new(0.5, 0.5)).map(|v| v.round()) + Vec2::new(0.5, 0.5)
            } else {
                point.map(|v| v.round())
            }
        }

        let position = uv * screen;

        let origin = screen / 2.0 + self.offset;
        let grid_size = Vec2::new(self.grid_size, self.grid_size);
        let sub_grid_div = Vec2::new(self.subdivisions, self.subdivisions);

        let bg_color = Vec4::new(0.05, 0.05, 0.05, 1.0);
        let line_color = Vec4::new(0.15, 0.15, 0.15, 1.0);
        let sub_line_color = Vec4::new(0.11, 0.11, 0.11, 1.0);

        let th = 1.0;
        let sth = 1.0;

        let aligned_origin = align_pixel(origin, 1);
        let rel_p = position - aligned_origin;
        let dist = mul_dist(grid_size, rel_p);

        if dist.x.min(dist.y) <= th * 0.5 {
            return vec4_to_pixel(&line_color);
        }

        let dist_to_floor =
            (rel_p - grid_size * (rel_p / grid_size).map(|v| v.floor())).map(|v| v.abs());
        let sub_size = grid_size / sub_grid_div.map(|v: f32| v.round());

        let sub_dist = mul_dist(sub_size, dist_to_floor);

        // Number of columns and rows
        let rc = (dist / sub_size).map(|v| v.round());

        // Extra pixels for the last row/column
        let extra = grid_size - sub_size * sub_grid_div;

        let sub_dist = Vec2::new(
            if rc.x == sub_grid_div.x {
                sub_dist.x + extra.x
            } else {
                sub_dist.x
            },
            if rc.y == sub_grid_div.y {
                sub_dist.y + extra.y
            } else {
                sub_dist.y
            },
        );

        if sub_dist.x.min(sub_dist.y) <= sth * 0.5 {
            return vec4_to_pixel(&sub_line_color);
        }

        vec4_to_pixel(&bg_color)
    }
}
