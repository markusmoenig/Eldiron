use crate::prelude::*;
use theframework::prelude::*;

pub struct GameCanvas {
    pub canvas: TheRGBBuffer,
    pub sunlight_canvas: TheRGBBuffer,

    pub distance_canvas: TheFlattenedMap<half::f16>,
    // pub occlusion_canvas: TheFlattenedMap<half::f16>,
    // pub mat_type_canvas: TheFlattenedMap<u8>,
    // pub mat1_canvas: TheFlattenedMap<half::f16>,
    // pub mat2_canvas: TheFlattenedMap<half::f16>,
    // pub mat3_canvas: TheFlattenedMap<half::f16>,
    // pub normal_canvas: TheFlattenedMap<(half::f16, half::f16, half::f16)>,
    pub lights_canvas: TheFlattenedMap<Vec<PreRenderedLight>>,
}

impl Default for GameCanvas {
    fn default() -> Self {
        Self::empty()
    }
}

impl GameCanvas {
    pub fn empty() -> Self {
        Self {
            canvas: TheRGBBuffer::empty(),
            sunlight_canvas: TheRGBBuffer::empty(),

            distance_canvas: TheFlattenedMap::new(0, 0),
            // occlusion_canvas: TheFlattenedMap::new(0, 0),
            // mat_type_canvas: TheFlattenedMap::new(0, 0),
            // mat1_canvas: TheFlattenedMap::new(0, 0),
            // mat2_canvas: TheFlattenedMap::new(0, 0),
            // mat3_canvas: TheFlattenedMap::new(0, 0),
            // normal_canvas: TheFlattenedMap::new(0, 0),
            lights_canvas: TheFlattenedMap::new(0, 0),
        }
    }

    pub fn resize(&mut self, width: i32, height: i32) {
        self.canvas.resize(width, height);
        self.sunlight_canvas.resize(width, height);
        self.distance_canvas.resize(width, height);
        // self.occlusion_canvas.resize(width, height);
        // self.mat_type_canvas.resize(width, height);
        // self.mat1_canvas.resize(width, height);
        // self.mat2_canvas.resize(width, height);
        // self.mat3_canvas.resize(width, height);
        // self.normal_canvas.resize(width, height);
        self.lights_canvas.resize(width, height);
    }

    pub fn copy_into(&mut self, x: i32, y: i32, tile: &PreRenderedTileData) {
        self.canvas.copy_into(x, y, &tile.albedo);
        self.sunlight_canvas.copy_into(x, y, &tile.sunlight);
        self.distance_canvas.copy_into(x, y, &tile.distance);
        // self.occlusion_canvas.copy_into(x, y, &tile.occlusion);
        // self.mat_type_canvas.copy_into(x, y, &tile.mat_type);
        // self.mat1_canvas.copy_into(x, y, &tile.mat1);
        // self.mat2_canvas.copy_into(x, y, &tile.mat2);
        // self.mat3_canvas.copy_into(x, y, &tile.mat3);
        // self.normal_canvas.copy_into(x, y, &tile.normal);
        self.lights_canvas.copy_into(x, y, &tile.lights);
    }

    pub fn get_albedo(&self, x: i32, y: i32) -> Vec3f {
        let mut albedo = Vec3f::zero();
        if let Some(rgb) = self.canvas.get_pixel(x, y) {
            albedo.x = rgb[0] as f32 / 255.0;
            albedo.y = rgb[1] as f32 / 255.0;
            albedo.z = rgb[2] as f32 / 255.0;
        }
        albedo
    }

    pub fn get_sunlight(&self, x: i32, y: i32) -> Vec3f {
        let mut sunlight = Vec3f::zero();
        if let Some(rgb) = self.sunlight_canvas.get_pixel(x, y) {
            sunlight.x = rgb[0] as f32 / 255.0;
            sunlight.y = rgb[1] as f32 / 255.0;
            sunlight.z = rgb[2] as f32 / 255.0;
        }
        sunlight
    }

    pub fn get_distance(&self, x: i32, y: i32) -> f32 {
        let mut distance = 0.0;
        if let Some(dist) = self.distance_canvas.get((x, y)) {
            distance = dist.to_f32();
        }
        distance
    }

    /*
    pub fn get_occlusion(&self, x: i32, y: i32) -> f32 {
        let mut occlusion = 0.0;
        if let Some(occ) = self.occlusion_canvas.get((x, y)) {
            occlusion = occ.to_f32();
        }
        occlusion
    }

    pub fn get_material(&self, x: i32, y: i32) -> (u8, f32, f32, f32) {
        let mut material: (u8, f32, f32, f32) = (0, 0.0, 0.0, 0.0);
        if let Some(mat_type) = self.mat_type_canvas.get((x, y)) {
            material.0 = *mat_type;
        }
        if let Some(mat1) = self.mat1_canvas.get((x, y)) {
            material.1 = mat1.to_f32();
        }
        if let Some(mat2) = self.mat2_canvas.get((x, y)) {
            material.2 = mat2.to_f32();
        }
        if let Some(mat3) = self.mat3_canvas.get((x, y)) {
            material.3 = mat3.to_f32();
        }
        material
    }*/

    // pub fn get_normal(&self, x: i32, y: i32) -> Vec3f {
    //     let mut normal = Vec3f::zero();
    //     if let Some(dist) = &self.normal_canvas.get((x, y)) {
    //         normal.x = dist.0.to_f32();
    //         normal.y = dist.1.to_f32();
    //         normal.z = dist.2.to_f32();
    //     }
    //     normal
    // }
}
