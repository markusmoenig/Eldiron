use crate::prelude::*;
use theframework::prelude::*;

fn default_subdivisions() -> usize {
    1
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug, Copy)]
pub enum HeightmapInterpolation {
    Linear,
    Smoothstep,
    Step(f32),
}

use HeightmapInterpolation::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct Heightmap {
    #[serde(with = "vectorize")]
    pub data: FxHashMap<(i32, i32), Vec<f32>>,

    #[serde(default)]
    #[serde(with = "vectorize")]
    // Holds the RGB Color
    pub material_mask: FxHashMap<(i32, i32), TheRGBBuffer>,

    // Holds the Roughness, Metallic and Bump values.
    #[serde(default)]
    #[serde(with = "vectorize")]
    pub material_mask2: FxHashMap<(i32, i32), TheRGBBuffer>,

    #[serde(with = "vectorize")]
    pub interpolation: FxHashMap<(i32, i32), HeightmapInterpolation>,

    #[serde(default = "default_subdivisions")]
    pub subdivisions: usize,
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

            material_mask: FxHashMap::default(),
            material_mask2: FxHashMap::default(),

            interpolation: FxHashMap::default(),

            subdivisions: 1,
        }
    }

    pub fn set_material_mask(&mut self, x: i32, y: i32, material: TheRGBBuffer) {
        self.material_mask.insert((x, y), material);
    }

    pub fn get_material_mask(&self, x: i32, y: i32) -> Option<&TheRGBBuffer> {
        self.material_mask.get(&(x, y))
    }

    pub fn get_material_mask_mut(&mut self, x: i32, y: i32) -> Option<&mut TheRGBBuffer> {
        self.material_mask.get_mut(&(x, y))
    }

    pub fn set_material_mask2(&mut self, x: i32, y: i32, material: TheRGBBuffer) {
        self.material_mask2.insert((x, y), material);
    }

    pub fn get_material_mask2(&self, x: i32, y: i32) -> Option<&TheRGBBuffer> {
        self.material_mask2.get(&(x, y))
    }

    pub fn get_material_mask_mut2(&mut self, x: i32, y: i32) -> Option<&mut TheRGBBuffer> {
        self.material_mask2.get_mut(&(x, y))
    }

    // Get the height at a specific position (x, y) with f32 precision
    pub fn get_height(&self, x: f32, y: f32) -> f32 {
        let (tile_x, tile_y, sub_x, sub_y) = self.calculate_tile_and_subdivision(x, y);

        if let Some(subtile_data) = self.data.get(&(tile_x, tile_y)) {
            let index = self.subdivision_index(sub_x, sub_y);
            subtile_data.get(index).cloned().unwrap_or(0.0)
        } else {
            0.0
        }
    }

    // Set the height at a specific position (x, y) with f32 precision
    pub fn set_height(&mut self, x: f32, y: f32, height: f32) {
        let (tile_x, tile_y, sub_x, sub_y) = self.calculate_tile_and_subdivision(x, y);

        let index = self.subdivision_index(sub_x, sub_y);
        let tile_data =
            self.data
                .entry((tile_x, tile_y))
                .or_insert(vec![0.0; self.subdivisions * self.subdivisions]);
        tile_data[index] = height;
    }

    // Get the height for a specific subdivision in a tile
    // pub fn get_height_in_tile(&self, tile_x: i32, tile_y: i32, sub_x: usize, sub_y: usize) -> f32 {
    //     let index = self.subdivision_index(sub_x, sub_y);
    //     if let Some(tile_data) = self.data.get(&(tile_x, tile_y)) {
    //         tile_data.get(index).cloned().unwrap_or(0.0)
    //     } else {
    //         0.0 // Default height value if the tile does not exist
    //     }
    // }

    /// Set the height for a specific subdivision in a tile
    pub fn set_height_in_tile(
        &mut self,
        tile_x: i32,
        tile_y: i32,
        sub_x: usize,
        sub_y: usize,
        height: f32,
    ) {
        let index = self.subdivision_index(sub_x, sub_y);
        let tile_data =
            self.data
                .entry((tile_x, tile_y))
                .or_insert(vec![0.0; self.subdivisions * self.subdivisions]);
        tile_data[index] = height;
    }

    // Calculate the tile and subdivision index based on the f32 x and y coordinates
    pub fn calculate_tile_and_subdivision(&self, x: f32, y: f32) -> (i32, i32, usize, usize) {
        let tile_x = x.floor() as i32;
        let tile_y = y.floor() as i32;

        let fractional_x = x - x.floor();
        let fractional_y = y - y.floor();

        let sub_x = (fractional_x * self.subdivisions as f32).floor() as usize;
        let sub_y = (fractional_y * self.subdivisions as f32).floor() as usize;

        (tile_x, tile_y, sub_x, sub_y)
    }

    // Convert (sub_x, sub_y) into a linear index for the Vec
    fn subdivision_index(&self, sub_x: usize, sub_y: usize) -> usize {
        sub_y * self.subdivisions + sub_x
    }

    pub fn set_interpolation(&mut self, x: i32, y: i32, inter: HeightmapInterpolation) {
        self.interpolation.insert((x, y), inter);
    }

    pub fn get_interpolation(&self, x: i32, y: i32) -> HeightmapInterpolation {
        *self.interpolation.get(&(x, y)).unwrap_or(&Smoothstep)
    }

    #[inline(always)]
    fn step_interpolate(value: f32, step_size: f32) -> f32 {
        (value / step_size).floor() * step_size
    }

    /*
    pub fn interpolate_height(&self, x: f32, y: f32) -> f32 {
        let x0 = x.floor() as i32;
        let x1 = x0 + 1;
        let y0 = y.floor() as i32;
        let y1 = y0 + 1;

        let h00 = self.get_height(x0 as f32, y0 as f32);
        let h10 = self.get_height(x1 as f32, y0 as f32);
        let h01 = self.get_height(x0 as f32, y1 as f32);
        let h11 = self.get_height(x1 as f32, y1 as f32);

        let tx = x - x0 as f32;
        let ty = y - y0 as f32;

        match self.get_interpolation(x0, y0) {
            Linear => {
                let h0 = h00 * (1.0 - tx) + h10 * tx;
                let h1 = h01 * (1.0 - tx) + h11 * tx;
                h0 * (1.0 - ty) + h1 * ty
            }
            Smoothstep => {
                let tx = smoothstep(0.0, 1.0, tx);
                let ty = smoothstep(0.0, 1.0, ty);
                let h0 = h00 * (1.0 - tx) + h10 * tx;
                let h1 = h01 * (1.0 - tx) + h11 * tx;
                h0 * (1.0 - ty) + h1 * ty
            }
            Step(step_size) => {
                let tx = Self::step_interpolate(tx, step_size);
                let ty = Self::step_interpolate(ty, step_size);
                let h0 = h00 * (1.0 - tx) + h10 * tx;
                let h1 = h01 * (1.0 - tx) + h11 * tx;
                h0 * (1.0 - ty) + h1 * ty
            }
        }
    }*/

    pub fn interpolate_height(&self, x: f32, y: f32) -> f32 {
        // Calculate which tile and subdivision we are in for the given coordinates
        let tile_x = x.floor() as i32;
        let tile_y = y.floor() as i32;

        // Determine the fractional part inside the tile (used for subdivision index calculation)
        let fractional_x = x - tile_x as f32;
        let fractional_y = y - tile_y as f32;

        // Calculate the subdivision indices within the tile
        let sub_x = (fractional_x * self.subdivisions as f32).floor() as usize;
        let sub_x_next = (sub_x + 1) % self.subdivisions;

        let sub_y = (fractional_y * self.subdivisions as f32).floor() as usize;
        let sub_y_next = (sub_y + 1) % self.subdivisions;

        // Handle tile transitions
        let tile_x_next = if sub_x_next == 0 { tile_x + 1 } else { tile_x };
        let tile_y_next = if sub_y_next == 0 { tile_y + 1 } else { tile_y };

        // Get the heights of the four surrounding subdivisions, handling tile transitions
        let h00 = self.get_height_in_tile(tile_x, tile_y, sub_x, sub_y); // Bottom-left subdivision
        let h10 = self.get_height_in_tile(tile_x_next, tile_y, sub_x_next, sub_y); // Bottom-right subdivision
        let h01 = self.get_height_in_tile(tile_x, tile_y_next, sub_x, sub_y_next); // Top-left subdivision
        let h11 = self.get_height_in_tile(tile_x_next, tile_y_next, sub_x_next, sub_y_next); // Top-right subdivision

        // Calculate interpolation fractions based on the subdivision position inside the tile
        let tx = fractional_x * self.subdivisions as f32 - sub_x as f32;
        let ty = fractional_y * self.subdivisions as f32 - sub_y as f32;

        // Perform interpolation based on the interpolation method defined at (tile_x, tile_y)
        match self.get_interpolation(tile_x, tile_y) {
            HeightmapInterpolation::Linear => {
                // Bilinear interpolation
                let h0 = h00 * (1.0 - tx) + h10 * tx;
                let h1 = h01 * (1.0 - tx) + h11 * tx;
                h0 * (1.0 - ty) + h1 * ty
            }
            HeightmapInterpolation::Smoothstep => {
                // Smoothstep interpolation
                let tx = smoothstep(0.0, 1.0, tx);
                let ty = smoothstep(0.0, 1.0, ty);
                let h0 = h00 * (1.0 - tx) + h10 * tx;
                let h1 = h01 * (1.0 - tx) + h11 * tx;
                h0 * (1.0 - ty) + h1 * ty
            }
            HeightmapInterpolation::Step(step_size) => {
                // Step interpolation
                let tx = Self::step_interpolate(tx, step_size);
                let ty = Self::step_interpolate(ty, step_size);
                let h0 = h00 * (1.0 - tx) + h10 * tx;
                let h1 = h01 * (1.0 - tx) + h11 * tx;
                h0 * (1.0 - ty) + h1 * ty
            }
        }
    }

    fn get_height_in_tile(&self, tile_x: i32, tile_y: i32, sub_x: usize, sub_y: usize) -> f32 {
        self.get_height(
            tile_x as f32 + (sub_x as f32 / self.subdivisions as f32),
            tile_y as f32 + (sub_y as f32 / self.subdivisions as f32),
        )
    }

    pub fn calculate_normal(&self, x: f32, y: f32, epsilon: f32) -> Vec3f {
        let height = self.interpolate_height(x, y);
        let height_dx = self.interpolate_height(x + epsilon, y);
        let height_dy = self.interpolate_height(x, y + epsilon);

        let dx = (height_dx - height) / epsilon;
        let dy = (height_dy - height) / epsilon;

        normalize(vec3f(-dx, 1.0, -dy))
    }

    /// Get the bump of the material (if any) at the given position.
    pub fn get_material_bump(&self, p: Vec3f) -> Option<f32> {
        let tile = vec2i(p.x.floor() as i32, p.z.floor() as i32);

        if let Some(mask) = self.get_material_mask2(tile.x, tile.y) {
            if let Some(material_mask) = mask.at_f(vec2f(p.x.fract(), p.z.fract())) {
                return Some(material_mask[2] as f32 / 255.0);
            }
        }
        None
    }

    /// Calculates the normal at the given position and evaluates material bumps.
    pub fn calculate_normal_with_material(&self, p: Vec3f, epsilon: f32) -> Vec3f {
        let x = p.x;
        let y = p.z;

        let mut height = self.interpolate_height(x, y);
        if let Some(bump) = self.get_material_bump(p) {
            height -= bump;
        }

        let mut height_dx = self.interpolate_height(x + epsilon, y);
        if let Some(bump) = self.get_material_bump(vec3f(x + epsilon, 0.0, y)) {
            height_dx -= bump;
        }

        let mut height_dy = self.interpolate_height(x, y + epsilon);
        if let Some(bump) = self.get_material_bump(vec3f(x, 0.0, y + epsilon)) {
            height_dy -= bump;
        }

        let dx = (height_dx - height) / epsilon;
        let dy = (height_dy - height) / epsilon;

        normalize(vec3f(-dx, 1.0, -dy))
    }

    /// Raymarches the terrain and returns the distance.
    pub fn raymarch(&self, ray: &Ray) -> Option<f32> {
        let mut t = 0.0;

        for _ in 0..30 {
            //while t < max_dist {
            let pos = ray.at(t);

            let height = self.interpolate_height(pos.x, pos.z);
            let d = pos.y - height;

            if d.abs() < 0.0001 {
                return Some(t);
            }

            t += d;
        }

        None
    }

    /// Returns the distance at the given position
    pub fn distance(&self, p: Vec3f) -> f32 {
        let mut bump = 0.0;

        let height = self.interpolate_height(p.x, p.z);
        let tile = vec2i(p.x.floor() as i32, p.z.floor() as i32);

        if let Some(mask) = self.get_material_mask2(tile.x, tile.y) {
            if let Some(material_mask) = mask.at_f(vec2f(p.x.fract(), p.z.fract())) {
                bump = material_mask[2] as f32 / 255.0;
            }
        }

        p.y - height - bump
    }

    /// Raymarches the terrain and evaluates materials and bumps.
    pub fn compute_hit(&self, ray: &Ray, hit: &mut Hit) -> Option<f32> {
        let mut t = 0.0;

        for _ in 0..150 {
            let p = ray.at(t);

            let mut bump = 0.0;

            let height = self.interpolate_height(p.x, p.z);
            let tile = vec2i(p.x.floor() as i32, p.z.floor() as i32);

            if let Some(mask) = self.get_material_mask2(tile.x, tile.y) {
                if let Some(material_mask) = mask.at_f(vec2f(p.x.fract(), p.z.fract())) {
                    bump = material_mask[2] as f32 / 255.0;
                }
            }
            /*
            if let Some(mask) = self.get_material_mask(tile.x, tile.y) {
                if let Some(material_mask) = mask.at_f(vec2f(p.x.fract(), p.z.fract())) {
                    let m = material_mask[0];
                    if m > 0 {
                        let index = (m - 1) as usize;
                        if let Some((_id, material)) = materials.get_index(index) {
                            if let Some(m_params) = material_params.get(&material.id) {
                                hit.global_uv = vec2f(p.x, p.z);
                                hit.pattern_pos = hit.global_uv;

                                hit.mode = HitMode::Bump;
                                material.follow_trail(0, 0, hit, palette, textures, m_params);
                                bump = hit.bump;

                                has_material_hit = true;
                            }
                        }
                    }
                }
            }*/

            let d = p.y - height - bump;

            if d < 0.0001 {
                if let Some(mask) = self.get_material_mask(tile.x, tile.y) {
                    if let Some(material_mask) = mask.at_f(vec2f(p.x.fract(), p.z.fract())) {
                        hit.mat.base_color = TheColor::from_u8_array_3(material_mask).to_vec3f();

                        if let Some(mask2) = self.get_material_mask2(tile.x, tile.y) {
                            if let Some(material_mask2) =
                                mask2.at_f(vec2f(p.x.fract(), p.z.fract()))
                            {
                                hit.mat.roughness = material_mask2[0] as f32 / 255.0;
                                hit.mat.metallic = material_mask2[1] as f32 / 255.0;
                            }
                        }

                        hit.hit_point = p;
                        hit.global_uv = vec2f(p.x, p.z);
                        hit.pattern_pos = hit.global_uv;
                        hit.uv = vec2f(p.x.fract(), p.z.fract());
                        hit.normal = self.calculate_normal_with_material(p, 0.001);
                        hit.is_valid = true;
                    } else {
                        return None;
                    }
                } else {
                    return None;
                }

                return Some(t);

                /*
                if has_material_hit {
                    hit.hit_point = p;
                    hit.global_uv = vec2f(p.x, p.z);
                    hit.pattern_pos = hit.global_uv;
                    hit.uv = vec2f(p.x.fract(), p.z.fract());
                    hit.normal = self.calculate_normal_with_material(
                        p,
                        palette,
                        textures,
                        materials,
                        material_params,
                    );

                    if let Some(mask) = self.get_material_mask(tile.x, tile.y) {
                        if let Some(material_mask) = mask.at_f(vec2f(p.x.fract(), p.z.fract())) {
                            let m = material_mask[0];
                            if m > 0 {
                                let index = (m - 1) as usize;
                                if let Some((_id, material)) = materials.get_index(index) {
                                    if let Some(m_params) = material_params.get(&material.id) {
                                        material.compute(hit, palette, textures, m_params);
                                    }
                                }
                            }

                            // Overlay the 2nd material
                            let m = material_mask[1];
                            if m > 0 {
                                let index = (m - 1) as usize;
                                if let Some((_id, material)) = materials.get_index(index) {
                                    let mut mat_obj_params: Vec<Vec<f32>> = vec![];

                                    if let Some(m_params) = material_params.get(&material.id) {
                                        mat_obj_params.clone_from(m_params);
                                    }

                                    let t = material_mask[2] as f32 / 255.0;

                                    let mut h = hit.clone();
                                    material.compute(&mut h, palette, textures, &mat_obj_params);
                                    hit.mat.mix(&hit.mat.clone(), &h.mat, t);
                                }
                            }
                        }
                    }

                    return Some(t);
                } else {
                    return None;
                }*/
            }

            t += d * 0.5;
        }

        None
    }

    /*
    pub fn get_pixel_at(
        &self,
        coord: Vec2f,
        palette: &ThePalette,
        textures: &FxHashMap<Uuid, TheRGBATile>,
        materials: &IndexMap<Uuid, MaterialFXObject>,
        material_params: &FxHashMap<Uuid, Vec<Vec<f32>>>,
        material_index: usize,
    ) {
        let mut hit = Hit::default();

        hit.pattern_pos = coord;
        hit.global_uv = coord;

        if let Some(material_index) = self.get_material_index_at(p) {
            if let Some(material_params) = self.material_params.get(&material_id) {
                material.compute(
                    &mut hit,
                    &self.palette,
                    &TILEDRAWER.lock().unwrap().tiles,
                    material_params,
                );

                let pixel = TheColor::from(hit.mat.base_color).to_u8_array();
                b.set_pixel(x, y, &pixel);
                terrain_editor.buffer.set_pixel(x, y, &pixel);
            }
        }
    }*/
}
