use crate::collision_world::ChunkCollision;
use crate::{BBox, Batch2D, Batch3D, BillboardAnimation, CompiledLight};
use scenevm::GeoId;
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

    // Lights
    pub lights: Vec<CompiledLight>,

    // Occluded Sectors
    pub occluded_sectors: Vec<(BBox, f32)>,

    // Collision
    pub collision: ChunkCollision,

    // Billboards (temporarily stored during build, transferred to SceneHandler)
    pub billboards: Vec<BillboardMetadata>,
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
            lights: vec![],
            occluded_sectors: vec![],
            collision: ChunkCollision::new(),
            billboards: vec![],
        }
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
