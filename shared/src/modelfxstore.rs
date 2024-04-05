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
}
