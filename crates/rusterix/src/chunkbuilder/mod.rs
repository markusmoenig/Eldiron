pub mod action;
pub mod d2chunkbuilder;
pub mod d3chunkbuilder;
pub mod surface_mesh_builder;
pub mod terrain_generator;

use crate::collision_world::ChunkCollision;
use crate::{Assets, Chunk, Map};
use vek::Vec2;

/// The ChunkBuilder Trait
#[allow(unused)]
pub trait ChunkBuilder: Send + Sync {
    fn new() -> Self
    where
        Self: Sized;

    fn build(
        &mut self,
        map: &Map,
        assets: &Assets,
        chunk: &mut Chunk,
        vmchunk: &mut scenevm::Chunk,
    ) {
    }

    /// Build only collision geometry (for server-side use, no rendering)
    fn build_collision(
        &mut self,
        map: &Map,
        chunk_origin: Vec2<i32>,
        chunk_size: i32,
    ) -> ChunkCollision {
        ChunkCollision::new()
    }

    fn boxed_clone(&self) -> Box<dyn ChunkBuilder>;
}
