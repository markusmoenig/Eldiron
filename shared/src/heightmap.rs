use crate::prelude::*;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Heightmap {
    data: FxHashMap<(i32, i32), f32>,
}

impl Default for Heightmap {
    fn default() -> Self {
        Self::new()
    }
}

impl Heightmap {
    pub fn new() -> Self {
        Self {
            data: FxHashMap::default(),
        }
    }

    pub fn set_height(&mut self, x: i32, y: i32, height: f32) {
        self.data.insert((x, y), height);
    }

    pub fn get_height(&self, x: i32, y: i32) -> f32 {
        *self.data.get(&(x, y)).unwrap_or(&0.0)
    }

    // fn cubic_interpolate(p0: f32, p1: f32, p2: f32, p3: f32, t: f32) -> f32 {
    //     let a = -0.5 * p0 + 1.5 * p1 - 1.5 * p2 + 0.5 * p3;
    //     let b = p0 - 2.5 * p1 + 2.0 * p2 - 0.5 * p3;
    //     let c = -0.5 * p0 + 0.5 * p2;
    //     let d = p1;
    //     a * t * t * t + b * t * t + c * t + d
    // }

    // pub fn interpolate_height(&self, x: f32, y: f32) -> f32 {
    //     let x0 = x.floor() as i32 - 1;
    //     let y0 = y.floor() as i32 - 1;

    //     let mut patch = [[0.0; 4]; 4];
    //     for (j, row) in patch.iter_mut().enumerate() {
    //         for (i, cell) in row.iter_mut().enumerate() {
    //             *cell = self.get_height(x0 + i as i32, y0 + j as i32);
    //         }
    //     }

    //     let tx = x - x.floor();
    //     let ty = y - y.floor();

    //     let mut col = [0.0; 4];
    //     for (i, col_val) in col.iter_mut().enumerate() {
    //         *col_val =
    //             Self::cubic_interpolate(patch[0][i], patch[1][i], patch[2][i], patch[3][i], ty);
    //     }
    //     Self::cubic_interpolate(col[0], col[1], col[2], col[3], tx)
    // }

    pub fn interpolate_height(&self, x: f32, y: f32) -> f32 {
        // Get the base tile indices
        let x0 = x.floor() as i32;
        let x1 = x0 + 1;
        let y0 = y.floor() as i32;
        let y1 = y0 + 1;

        // fn step_interpolate(value: f32, step_size: f32) -> f32 {
        //     (value / step_size).floor() * step_size
        // }
        // Get the fractional parts

        let frac_x = x - x.floor();
        let frac_y = y - y.floor();

        // let frac_x = smoothstep(0.0, 1.0, x - x.floor());
        // let frac_y = smoothstep(0.0, 1.0, y - y.floor());

        // let frac_x = step_interpolate(x - x0 as f32, 0.2);
        // let frac_y = step_interpolate(y - y0 as f32, 0.2);

        // Get the heights at the four corners
        let h00 = self.get_height(x0, y0);
        let h10 = self.get_height(x1, y0);
        let h01 = self.get_height(x0, y1);
        let h11 = self.get_height(x1, y1);

        // Perform bilinear interpolation
        let h0 = h00 * (1.0 - frac_x) + h10 * frac_x;
        let h1 = h01 * (1.0 - frac_x) + h11 * frac_x;
        h0 * (1.0 - frac_y) + h1 * frac_y
    }

    pub fn calculate_normal(&self, x: f32, y: f32, epsilon: f32) -> Vec3f {
        let height = self.interpolate_height(x, y);
        let height_dx = self.interpolate_height(x + epsilon, y);
        let height_dy = self.interpolate_height(x, y + epsilon);

        let dx = (height_dx - height) / epsilon;
        let dy = (height_dy - height) / epsilon;

        normalize(vec3f(-dx, 1.0, -dy))
    }

    pub fn raymarch(&self, ray: &Ray) -> Option<f32> {
        let mut t = 0.0;

        for _ in 0..20 {
            //while t < max_dist {
            let pos = ray.at(t);

            let height = self.interpolate_height(pos.x, pos.z);

            // if pos.y < height {
            //     return Some(t);
            // }

            // Calculate the dynamic step size
            let step_size = pos.y - height; // * 0.05;

            if step_size.abs() < 0.0001 {
                return Some(t);
            }

            t += step_size;
        }

        None
    }
}
