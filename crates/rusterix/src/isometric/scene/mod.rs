//! Camera-independent isometric scene selection and borrowed source placement.
//! This module owns no viewport pixels, GPU device, runtime or presentation surface.
use super::{AvatarSprite, flat_terrain_contains, item_cutaway, item_sprite_key, lighting};
use crate::{Assets, Map, Sector, Tile, Value, chunkbuilder::terrain_surface::TerrainSurface};
use std::{collections::HashMap, sync::Arc};
use uuid::Uuid;
use vek::{Vec2, Vec3};
mod collect;
mod contact;
pub use contact::ContactPolicy;
mod source;
pub use source::{Sprite, SpriteImage, SpriteOwner};

/// Geometry caches shared by native GPU production and the reference adapter.
/// No viewport-sized albedo, surface, depth or ownership array belongs here.
#[derive(Default)]
pub struct SceneCache {
    terrain_key: Option<(Uuid, u32)>,
    terrain_model: Option<Arc<TerrainSurface>>,
    pub(super) shadow_geometry: Option<(u64, Arc<scenevm::isometric::ShadowGeometry>)>,
    terrain_shadow: Option<((Uuid, u32), Arc<scenevm::isometric::ShadowGeometry>)>,
}

/// A scene frame borrowing authored/derived asset sources in reference painter order.
/// Geometry and sprite bytes are source assets; none are a composited viewport.
pub struct ScenePlan<'a> {
    pub terrain: Option<Arc<TerrainSurface>>,
    pub ground: Vec<Sprite<'a>>,
    pub upright: Vec<Sprite<'a>>,
    pub floors: Vec<(&'a Sector, &'a Tile)>,
    pub casters: Vec<lighting::Caster<'a>>,
    mesh_instances: Vec<(&'a scenevm::isometric::ShadowGeometry, Vec3<f32>, f32)>,
}

impl SceneCache {
    fn terrain(&mut self, map: &Map) -> Option<Arc<TerrainSurface>> {
        let key = (map.id, map.changed);
        if self.terrain_key != Some(key) {
            self.terrain_key = Some(key);
            self.terrain_model = TerrainSurface::from_map(map).map(Arc::new);
        }
        self.terrain_model.clone()
    }

    pub fn prepare<'a>(
        &mut self,
        map: &'a Map,
        assets: &'a Assets,
        seconds: f32,
        avatars: &'a HashMap<usize, AvatarSprite>,
    ) -> ScenePlan<'a> {
        collect::collect(map, assets, seconds, avatars, self.terrain(map))
    }

    /// Reuses static terrain triangles while updating transformed dynamic meshes.
    pub fn shadow_geometry(
        &mut self,
        map: &Map,
        assets: &Assets,
        plan: &ScenePlan<'_>,
    ) -> Option<Arc<scenevm::isometric::ShadowGeometry>> {
        let mesh_instances = &plan.mesh_instances;
        let terrain_model = &plan.terrain;
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        (map.id, map.changed, assets.tiles_revision).hash(&mut hasher);
        for (mesh, pos, scale) in mesh_instances {
            (
                mesh.vertices.as_ptr() as usize,
                mesh.vertices.len(),
                pos.x.to_bits(),
                pos.y.to_bits(),
                pos.z.to_bits(),
                scale.to_bits(),
            )
                .hash(&mut hasher);
        }
        let key = hasher.finish();
        let cached = self
            .shadow_geometry
            .as_ref()
            .filter(|c| c.0 == key)
            .map(|c| c.1.clone());
        let geometry = cached.unwrap_or_else(|| {
            let mut geometry = scenevm::isometric::ShadowGeometry {
                revision: key,
                ..Default::default()
            };
            if let Some(terrain) = &terrain_model {
                let terrain_key = (map.id, map.changed);
                let saved = self
                    .terrain_shadow
                    .as_ref()
                    .filter(|c| c.0 == terrain_key)
                    .map(|c| c.1.clone());
                let triangles = saved.unwrap_or_else(|| {
                    let mut triangles = scenevm::isometric::ShadowGeometry::default();
                    terrain.triangles(terrain.bounds, |t| {
                        triangles.triangle(t.map(|p| [p.x, p.y, p.z]))
                    });
                    let triangles = std::sync::Arc::new(triangles);
                    self.terrain_shadow = Some((terrain_key, triangles.clone()));
                    triangles
                });
                geometry.append(&triangles, [0.; 3], 1.);
            }
            for (mesh, pos, scale) in mesh_instances {
                geometry.append(mesh, [pos.x, pos.y, pos.z], *scale);
            }
            let geometry = std::sync::Arc::new(geometry);
            self.shadow_geometry = Some((key, geometry.clone()));
            geometry
        });
        (!geometry.vertices.is_empty()).then_some(geometry)
    }
}
