use crate::collision_world::ChunkCollision;
use crate::{Assets, BBox, Batch2D, Batch3D, BillboardAnimation, CompiledLight, Pixel, Texture};
use rusteria::{Program, RenderBuffer, Rusteria};
use scenevm::GeoId;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use vek::{Vec2, Vec3};

/// Billboard metadata for dynamic rendering
#[derive(Clone, Debug)]
pub struct BillboardMetadata {
    pub geo_id: GeoId,
    pub tile_id: Uuid,
    pub center: Vec3<f32>,
    pub up: Vec3<f32>,
    pub right: Vec3<f32>,
    pub size: f32,
    pub animation: BillboardAnimation,
    pub repeat_mode: scenevm::RepeatMode,
}

/// A chunk of 2D and 3D batches which make up a Scene.
pub struct Chunk {
    pub origin: Vec2<i32>,
    pub size: i32,
    pub bbox: BBox,

    // Geometry
    pub batches2d: Vec<Batch2D>,
    pub batches3d_opacity: Vec<Batch3D>,
    pub batches3d: Vec<Batch3D>,

    // Terrain
    pub terrain_batch2d: Option<Batch2D>,
    pub terrain_batch3d: Option<Batch3D>,
    pub terrain_texture: Option<Texture>,

    // Lights
    pub lights: Vec<CompiledLight>,

    // Occluded Sectors
    pub occluded_sectors: Vec<(BBox, f32)>,

    // Collision
    pub collision: ChunkCollision,

    // Billboards (temporarily stored during build, transferred to SceneHandler)
    pub billboards: Vec<BillboardMetadata>,

    /// The list of shaders for the Batches
    pub shaders: Vec<Program>,

    pub shader_textures: Vec<Option<Texture>>,

    /// The list of shaders which have opacity
    pub shaders_with_opacity: Vec<bool>,
}

impl Chunk {
    /// Create an empty chunk at the given coordinate.
    pub fn new(origin: Vec2<i32>, size: i32) -> Self {
        let bbox = BBox::from_pos_size(origin.map(|v| v as f32), Vec2::broadcast(size as f32));
        Self {
            origin,
            size,
            bbox,
            batches2d: vec![],
            batches3d_opacity: vec![],
            batches3d: vec![],
            terrain_batch2d: None,
            terrain_batch3d: None,
            terrain_texture: None,
            lights: vec![],
            occluded_sectors: vec![],
            collision: ChunkCollision::new(),
            billboards: vec![],
            shaders: vec![],
            shader_textures: vec![],
            shaders_with_opacity: vec![],
        }
    }

    /// Add a shader
    pub fn add_shader(&mut self, code: &str, assets: &Assets) -> Option<usize> {
        if code.is_empty() {
            return None;
        };

        let mut rs: Rusteria = Rusteria::default();
        let _module = match rs.parse_str(code) {
            Ok(module) => match rs.compile(&module) {
                Ok(()) => module,
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

        let width = 64;
        let height = 64;

        let mut texture = None;

        if let Some(shade_index) = rs.context.program.shade_index {
            let mut rbuffer = Arc::new(Mutex::new(RenderBuffer::new(width, height)));
            // let t0 = rs.get_time();
            rs.shade(&mut rbuffer, shade_index, &assets.palette);
            // let t1 = rs.get_time();
            // println!("Rendered in {}ms", t1 - t0);

            let b = rbuffer.lock().unwrap().as_rgba_bytes();

            let mut tex = Texture::new(b, width, height);
            tex.generate_normals(true);

            texture = Some(tex);
        }

        let index = self.shaders.len();

        self.shaders_with_opacity
            .push(rs.context.program.shader_supports_opacity());
        self.shaders.push(rs.context.program.clone());
        self.shader_textures.push(texture);

        Some(index)
    }

    /// Sample the baked terrain texture at the given world position
    pub fn sample_terrain_texture(&self, world_pos: Vec2<f32>, scale: Vec2<f32>) -> Pixel {
        let local_x = (world_pos.x / scale.x) - self.origin.x as f32;
        let local_y = (world_pos.y / scale.y) - self.origin.y as f32;

        if let Some(texture) = &self.terrain_texture {
            let pixels_per_tile = texture.width as i32 / self.size;

            let pixel_x = local_x * pixels_per_tile as f32;
            let pixel_y = local_y * pixels_per_tile as f32;

            let px = pixel_x.floor().clamp(0.0, texture.width as f32 - 1.0) as u32;
            let py = pixel_y.floor().clamp(0.0, texture.height as f32 - 1.0) as u32;

            return texture.get_pixel(px, py);
        }
        [0, 0, 0, 0]
    }

    /// Returns the sector occlusion at the given position.
    pub fn get_occlusion(&self, at: Vec2<f32>) -> f32 {
        for (bbox, occlusion) in &self.occluded_sectors {
            if bbox.contains(at) {
                return *occlusion;
            }
        }
        1.0
    }
}
