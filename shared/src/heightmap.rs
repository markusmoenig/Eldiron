use crate::prelude::*;
use theframework::prelude::*;

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
    pub data: FxHashMap<(i32, i32), f32>,
    #[serde(default)]
    #[serde(with = "vectorize")]
    pub material_mask: FxHashMap<(i32, i32), TheRGBBuffer>,
    #[serde(with = "vectorize")]
    pub interpolation: FxHashMap<(i32, i32), HeightmapInterpolation>,
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
            interpolation: FxHashMap::default(),
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

    pub fn set_height(&mut self, x: i32, y: i32, height: f32) {
        self.data.insert((x, y), height);
    }

    pub fn get_height(&self, x: i32, y: i32) -> f32 {
        *self.data.get(&(x, y)).unwrap_or(&0.0)
    }

    pub fn set_interpolation(&mut self, x: i32, y: i32, inter: HeightmapInterpolation) {
        self.interpolation.insert((x, y), inter);
    }

    pub fn get_interpolation(&self, x: i32, y: i32) -> HeightmapInterpolation {
        *self.interpolation.get(&(x, y)).unwrap_or(&Linear)
    }

    #[inline(always)]
    fn step_interpolate(value: f32, step_size: f32) -> f32 {
        (value / step_size).floor() * step_size
    }

    pub fn interpolate_height(&self, x: f32, y: f32) -> f32 {
        let x0 = x.floor() as i32;
        let x1 = x0 + 1;
        let y0 = y.floor() as i32;
        let y1 = y0 + 1;

        let h00 = self.get_height(x0, y0);
        let h10 = self.get_height(x1, y0);
        let h01 = self.get_height(x0, y1);
        let h11 = self.get_height(x1, y1);

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
    #[allow(clippy::too_many_arguments)]
    pub fn get_material_bump(
        &self,
        p: Vec3f,
        palette: &ThePalette,
        textures: &FxHashMap<Uuid, TheRGBATile>,
        materials: &IndexMap<Uuid, MaterialFXObject>,
        material_params: &FxHashMap<Uuid, Vec<Vec<f32>>>,
    ) -> Option<(f32, Uuid)> {
        let tile = vec2i(p.x.floor() as i32, p.z.floor() as i32);
        let mut hit: Hit = Hit::default();

        if let Some(mask) = self.get_material_mask(tile.x, tile.y) {
            if let Some(material_mask) = mask.at_f(vec2f(p.x.fract(), p.z.fract())) {
                let index = (material_mask[0] - 1) as usize;
                if let Some((_id, material)) = materials.get_index(index) {
                    if let Some(m_params) = material_params.get(&material.id) {
                        hit.global_uv = vec2f(p.x, p.z);
                        hit.pattern_pos = hit.global_uv;

                        hit.mode = HitMode::Bump;
                        material.follow_trail(0, 0, &mut hit, palette, textures, m_params);
                        return Some((hit.bump, material.id));
                    }
                }
            }
        }

        None
    }

    /// Calculates the normal at the given position and evaluates material bumps.
    pub fn calculate_normal_with_material(
        &self,
        p: Vec3f,
        palette: &ThePalette,
        textures: &FxHashMap<Uuid, TheRGBATile>,
        materials: &IndexMap<Uuid, MaterialFXObject>,
        material_params: &FxHashMap<Uuid, Vec<Vec<f32>>>,
    ) -> Vec3f {
        let x = p.x;
        let y = p.z;
        let epsilon = 0.001;

        let mut height = self.interpolate_height(x, y);
        if let Some((bump, _)) =
            self.get_material_bump(p, palette, textures, materials, material_params)
        {
            height -= bump;
        }

        let mut height_dx = self.interpolate_height(x + epsilon, y);
        if let Some((bump, _)) = self.get_material_bump(
            vec3f(x + epsilon, 0.0, y),
            palette,
            textures,
            materials,
            material_params,
        ) {
            height_dx -= bump;
        }

        let mut height_dy = self.interpolate_height(x, y + epsilon);
        if let Some((bump, _)) = self.get_material_bump(
            vec3f(x, 0.0, y + epsilon),
            palette,
            textures,
            materials,
            material_params,
        ) {
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

    /// Raymarches the terrain and evaluates materials and bumps.
    pub fn compute_hit(
        &self,
        ray: &Ray,
        hit: &mut Hit,
        palette: &ThePalette,
        textures: &FxHashMap<Uuid, TheRGBATile>,
        materials: &IndexMap<Uuid, MaterialFXObject>,
        material_params: &FxHashMap<Uuid, Vec<Vec<f32>>>,
    ) -> Option<f32> {
        let mut t = 0.0;

        for _ in 0..60 {
            let p = ray.at(t);

            let mut bump = 0.0;

            let height = self.interpolate_height(p.x, p.z);
            let tile = vec2i(p.x.floor() as i32, p.z.floor() as i32);

            let mut has_material_hit = false;

            if let Some(mask) = self.get_material_mask(tile.x, tile.y) {
                if let Some(material_mask) = mask.at_f(vec2f(p.x.fract(), p.z.fract())) {
                    let index = (material_mask[0] - 1) as usize;
                    if let Some((_id, material)) = materials.get_index(index) {
                        if let Some(m_params) = material_params.get(&material.id) {
                            hit.global_uv = vec2f(p.x, p.z);
                            hit.pattern_pos = hit.global_uv;

                            //material.get_material_distance(0, hit, palette, textures, m_params);
                            hit.mode = HitMode::Bump;
                            material.follow_trail(0, 0, hit, palette, textures, m_params);
                            bump = hit.bump;

                            has_material_hit = true;
                        }
                    }
                }
            }

            let d = p.y - height - bump;

            if d.abs() < 0.0001 {
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
                            let index = (material_mask[0] - 1) as usize;
                            if let Some((_id, material)) = materials.get_index(index) {
                                if let Some(m_params) = material_params.get(&material.id) {
                                    material.compute(hit, palette, textures, m_params);
                                }
                            }
                        }
                    }

                    return Some(t);
                } else {
                    return None;
                }
            }

            t += d * 0.5;
        }

        None
    }
}
