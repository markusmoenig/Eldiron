use crate::prelude::*;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ModelFXStore {
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

    pub fn prerender(&self, face_size: u16, palette: &ThePalette) -> RenderedTile {
        let mut rendered_tile = RenderedTile::new(face_size);

        let size = 200;
        let ro = vec3f(2.0, 2.0, 2.0);
        let rd = vec3f(0.0, 0.0, 0.0);

        let camera = Camera::new(ro, rd, 160.0);

        for y in 0..size {
            for x in 0..size {
                let uv = vec2f(x as f32 / size as f32, y as f32 / size as f32);

                let ray =
                    camera.create_ortho_ray(uv, vec2f(size as f32, size as f32), vec2f(0.0, 0.0));

                if let Some(hit) = self.render(&ray, 3.0, Vec3f::zero(), palette) {
                    let (uv, face) = self.get_uv_face(hit.normal, hit.hit_point);

                    let face = &mut rendered_tile.faces[face as usize];

                    let hit_uv = Vec2i::from(uv * (face_size as f32 - 1.0));

                    let rendered = Rendered {
                        color: TheColor::from_vec4f(hit.color).to_u8_array(),
                        ..Default::default()
                    };

                    face.set_safe(hit_uv.x as u16, hit_uv.y as u16, rendered);
                }
            }
        }
        rendered_tile
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
}
