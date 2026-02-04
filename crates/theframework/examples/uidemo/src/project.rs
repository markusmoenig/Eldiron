//use crate::prelude::*;
//use theframework::prelude::*;
use rust_pathtracer::prelude::*;

#[derive(Clone, Debug)]
pub struct Project {
    pub material: Material,
}

impl Default for Project {
    fn default() -> Self {
        Self::new()
    }
}

impl Project {
    pub fn new() -> Self {
        let mut material = Material::new();
        material.rgb = F3::new(1.0, 0.186, 0.0);
        material.metallic = 0.0;
        material.clearcoat = 1.0;
        material.clearcoat_gloss = 1.0;
        material.roughness = 0.1;
        Self { material }
    }
}
