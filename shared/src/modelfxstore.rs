use crate::prelude::*;
use theframework::prelude::*;

fn default_density() -> u8 {
    24
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ModelFXStore {
    /// The density of the voxel grid.
    #[serde(default = "default_density")]
    pub density: u8,

    pub floor: ModelFX,
    pub wall: ModelFX,
    pub ceiling: ModelFX,
}

impl Default for ModelFXStore {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelFXStore {
    pub fn new() -> Self {
        Self {
            density: default_density(),
            floor: ModelFX::default(),
            wall: ModelFX::default(),
            ceiling: ModelFX::default(),
        }
    }

    /// Render the model by checking the floor, wall, and ceiling.
    pub fn render(
        &self,
        ray: &Ray,
        max_distance: f32,
        key: Vec3f,
        palette: &ThePalette,
    ) -> Option<Hit> {
        let mut hit = None;
        let mut distance = std::f32::MAX;

        if let Some(floor_hit) = self.floor.render(ray, max_distance, key, palette) {
            if floor_hit.distance < distance {
                distance = floor_hit.distance;
                hit = Some(floor_hit);
            }
        }

        if let Some(wall_hit) = self.wall.render(ray, max_distance, key, palette) {
            if wall_hit.distance < distance {
                distance = wall_hit.distance;
                hit = Some(wall_hit);
            }
        }

        if let Some(ceiling_hit) = self.ceiling.render(ray, max_distance, key, palette) {
            if ceiling_hit.distance < distance {
                hit = Some(ceiling_hit);
            }
        }

        hit
    }

    /// Create the voxel model for the floor, wall, and ceiling.
    pub fn create_voxels(&mut self, density: u8, key: &Vec3f, palette: &ThePalette) {
        self.density = density;
        self.floor.create_voxels(density, key, palette);
        self.wall.create_voxels(density, key, palette);
        self.ceiling.create_voxels(density, key, palette);
    }

    /// Voxel dda
    pub fn dda(&self, ray: &Ray, wallfx: Vec3i) -> Option<Hit> {
        fn equal(l: f32, r: Vec3f) -> Vec3f {
            vec3f(
                if l == r.x { 1.0 } else { 0.0 },
                if l == r.y { 1.0 } else { 0.0 },
                if l == r.z { 1.0 } else { 0.0 },
            )
        }
        let density_f = self.density as f32;

        let mut ro = ray.o;
        ro *= self.density as f32;
        let rd = ray.d;

        let mut dist = 0.0;

        let max_y = max(self.floor.max_y_voxel, self.wall.max_y_voxel);

        // Check if the ray hits the plane of the max_y voxel.If yes, advance the ro to the plane.
        // If not we do not need to render as the ray passes over the voxel grid.
        // This optimization is especially important for first person view.
        if max_y < self.density - 1 {
            let plane_normal = vec3f(0.0, 1.0, 0.0);
            let denom = dot(plane_normal, rd);

            if denom.abs() > 0.0001 {
                let t = dot(vec3f(0.0, max_y as f32, 0.0) - ro, plane_normal) / denom;
                if t >= 0.0 {
                    ro += rd * t;
                    dist = t;
                } else {
                    return None;
                }
            }
        }

        let mut i = floor(ro);

        let mut normal = vec3f(0.0, -1.0, 0.0);
        let srd = signum(rd);

        let rdi = 1.0 / (2.0 * rd);

        loop {
            if i.x < 0.0
                || i.y < 0.0
                || i.z < 0.0
                || i.x > density_f
                || i.y > density_f
                || i.z > density_f
            {
                break;
            }

            let x = i.x as u8;
            let y = i.y as u8;
            let z = i.z as u8;

            if let Some(voxel) = self.wall.voxels.get(&(x, y + wallfx.y as u8, z)) {
                let mut hit = Hit::default();
                let c =
                    TheColor::from_u8_array([voxel.color[0], voxel.color[1], voxel.color[2], 255]);
                hit.color = c.to_vec4f();
                hit.roughness = (voxel.roughness as f32) / 255.0;
                hit.metallic = (voxel.metallic as f32) / 255.0;
                hit.reflectance = (voxel.reflectance as f32) / 255.0;
                hit.distance = dist / density_f;
                hit.normal = normal;
                return Some(hit);
            }

            if let Some(voxel) = self.floor.voxels.get(&(x, y, z)) {
                let mut hit = Hit::default();
                let c =
                    TheColor::from_u8_array([voxel.color[0], voxel.color[1], voxel.color[2], 255]);
                hit.color = c.to_vec4f();
                hit.roughness = (voxel.roughness as f32) / 255.0;
                hit.metallic = (voxel.metallic as f32) / 255.0;
                hit.reflectance = (voxel.reflectance as f32) / 255.0;
                hit.distance = dist / density_f;
                hit.normal = normal;
                return Some(hit);
            }

            let plain = (1.0 + srd - 2.0 * (ro - i)) * rdi;
            dist = min(plain.x, min(plain.y, plain.z));
            normal = equal(dist, plain) * srd;
            i += normal;
        }
        None
    }
}
