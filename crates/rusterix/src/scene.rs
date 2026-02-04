use crate::{Batch2D, Batch3D, Chunk, CompiledLight, HitInfo, MapMini, Ray, Shader, Tile};
use rayon::prelude::*;
use rusteria::{Program, Rusteria};
use theframework::prelude::*;
use vek::{Mat3, Mat4};

/// A scene of 2D and 3D batches which are passed to the rasterizer for rasterization.
pub struct Scene {
    /// Background shader
    pub background: Option<Box<dyn Shader>>,

    /// The lights in the scene
    pub lights: Vec<CompiledLight>,

    /// The lights in the scene
    pub dynamic_lights: Vec<CompiledLight>,

    /// 3D static batches which never change. Only use for scene with no async chunc rendering.
    pub d3_static: Vec<Batch3D>,

    /// 3D dynamic batches which can be updated dynamically.
    pub d3_dynamic: Vec<Batch3D>,

    /// 3D overlay batches.
    pub d3_overlay: Vec<Batch3D>,

    /// The 2D batches get rendered on top of the 3D batches (2D game or UI).
    /// Static 2D batches.
    pub d2_static: Vec<Batch2D>,
    /// 2D dynamic batches which can be updated dynamically.
    pub d2_dynamic: Vec<Batch2D>,

    /// The list of textures which the d3_dynamic batches index into.
    pub dynamic_textures: Vec<Tile>,

    /// The current animation frame
    pub animation_frame: usize,

    /// For 2D grid conversion when we dont use a matrix
    pub mapmini: MapMini,

    /// The list of shaders for the Batches
    pub shaders: Vec<Program>,

    /// The list of shaders which have opacity
    pub shaders_with_opacity: Vec<bool>,

    /// The build chunks
    pub chunks: FxHashMap<(i32, i32), Chunk>,
}

impl Default for Scene {
    fn default() -> Self {
        Scene::empty()
    }
}

impl Scene {
    // An empty scene
    pub fn empty() -> Self {
        Self {
            background: None,
            lights: vec![],
            dynamic_lights: vec![],
            d3_static: vec![],
            d3_dynamic: vec![],
            d3_overlay: vec![],
            d2_static: vec![],
            d2_dynamic: vec![],
            dynamic_textures: vec![],

            animation_frame: 1,

            mapmini: MapMini::default(),

            shaders: vec![],
            shaders_with_opacity: vec![],

            chunks: FxHashMap::default(),
        }
    }

    // From static 2D and 3D meshes.
    pub fn from_static(d2: Vec<Batch2D>, d3: Vec<Batch3D>) -> Self {
        Self {
            background: None,
            lights: vec![],
            dynamic_lights: vec![],
            d3_static: d3,
            d3_dynamic: vec![],
            d3_overlay: vec![],
            d2_static: d2,
            d2_dynamic: vec![],
            dynamic_textures: vec![],

            animation_frame: 1,

            mapmini: MapMini::default(),

            shaders: vec![],
            shaders_with_opacity: vec![],

            chunks: FxHashMap::default(),
        }
    }

    /// Add a shader
    pub fn add_shader(&mut self, code: &str) -> Option<usize> {
        if code.is_empty() {
            return None;
        };

        let mut rs: Rusteria = Rusteria::default();
        let _module = match rs.parse_str(code) {
            Ok(module) => match rs.compile(&module) {
                Ok(()) => {}
                Err(e) => {
                    eprintln!("Error compiling module: {e}");
                    return None;
                }
            },
            Err(e) => {
                eprintln!("Error parsing module: {e}");
                return None;
            }
        };

        let index = self.shaders.len();
        self.shaders_with_opacity
            .push(rs.context.program.shader_supports_opacity());
        self.shaders.push(rs.context.program.clone());

        Some(index)
    }

    /// Sets the background shader using the builder pattern.
    pub fn background(mut self, background: Box<dyn Shader>) -> Self {
        self.background = Some(background);
        self
    }

    /// Sets the lights using the builder pattern.
    pub fn lights(mut self, lights: Vec<CompiledLight>) -> Self {
        self.lights = lights;
        self
    }

    /// Increase the animation frame counter.
    pub fn anim_tick(&mut self) {
        self.animation_frame = self.animation_frame.wrapping_add(1);
    }

    /// Project the batches using the given matrices (which represent the global camera).
    pub fn project(
        &mut self,
        projection_matrix_2d: Option<Mat3<f32>>,
        view_matrix_3d: Mat4<f32>,
        projection_matrix_3d: Mat4<f32>,
        width: f32,
        height: f32,
    ) {
        self.chunks.par_iter_mut().for_each(|chunk| {
            for chunk2d in &mut chunk.1.batches2d {
                chunk2d.project(projection_matrix_2d);
            }
            if let Some(terrain2d) = &mut chunk.1.terrain_batch2d {
                terrain2d.project(projection_matrix_2d);
            }

            for chunk3d in &mut chunk.1.batches3d_opacity {
                chunk3d.clip_and_project(view_matrix_3d, projection_matrix_3d, width, height);
            }
            for chunk3d in &mut chunk.1.batches3d {
                chunk3d.clip_and_project(view_matrix_3d, projection_matrix_3d, width, height);
            }
            if let Some(terrain3d) = &mut chunk.1.terrain_batch3d {
                terrain3d.clip_and_project(view_matrix_3d, projection_matrix_3d, width, height);
            }
        });

        self.d2_static.par_iter_mut().for_each(|batch| {
            batch.project(projection_matrix_2d);
        });

        self.d2_dynamic.par_iter_mut().for_each(|batch| {
            batch.project(projection_matrix_2d);
        });

        self.d3_static.par_iter_mut().for_each(|batch| {
            batch.clip_and_project(view_matrix_3d, projection_matrix_3d, width, height);
        });

        self.d3_dynamic.par_iter_mut().for_each(|batch| {
            batch.clip_and_project(view_matrix_3d, projection_matrix_3d, width, height);
        });

        self.d3_overlay.iter_mut().for_each(|batch| {
            batch.clip_and_project(view_matrix_3d, projection_matrix_3d, width, height);
        });
    }

    /// Computes the normals for the static models
    pub fn compute_static_normals(&mut self) {
        self.d3_static.par_iter_mut().for_each(|batch| {
            batch.compute_vertex_normals();
        });
    }

    /// Computes the normals for the dynamic models
    pub fn compute_dynamic_normals(&mut self) {
        self.d3_dynamic.par_iter_mut().for_each(|batch| {
            batch.compute_vertex_normals();
        });
    }

    /// Intersect the ray with the scene.
    pub fn intersect(&self, ray: &Ray) -> HitInfo {
        let mut hitinfo = HitInfo::default();

        // Evaluate chunks
        for (_coord, chunk) in self.chunks.iter() {
            for batch in &chunk.batches3d_opacity {
                if let Some(hit) = batch.intersect(&ray, true) {
                    if hit.t < hitinfo.t {
                        hitinfo = hit;
                    }
                }
            }

            for batch in &chunk.batches3d {
                if let Some(hit) = batch.intersect(&ray, true) {
                    if hit.t < hitinfo.t {
                        if hit.profile_id.is_some() && hit.profile_id == hitinfo.profile_id {
                        } else {
                            hitinfo = hit;
                        }
                    }
                }
            }

            if let Some(batch) = &chunk.terrain_batch3d {
                if let Some(hit) = batch.intersect(&ray, true) {
                    if hit.t < hitinfo.t {
                        hitinfo = hit;
                    }
                }
            }
        }

        // Evaluate static
        for batch in self.d3_static.iter() {
            if let Some(hit) = batch.intersect(&ray, true) {
                if hit.t < hitinfo.t {
                    hitinfo = hit;
                }
            }
        }

        // Evaluate dynamic
        for batch in self.d3_dynamic.iter() {
            if let Some(hit) = batch.intersect(&ray, true) {
                if hit.t < hitinfo.t {
                    hitinfo = hit;
                }
            }
        }

        // Evaluate overlay
        for batch in self.d3_overlay.iter() {
            if let Some(hit) = batch.intersect(&ray, true) {
                hitinfo = hit;
            }
        }

        hitinfo
    }
}
