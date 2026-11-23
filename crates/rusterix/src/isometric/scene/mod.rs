//! Camera-independent isometric scene selection and borrowed source placement.
//! This module owns no viewport pixels, GPU device, runtime or presentation surface.
use super::{AvatarSprite, Projection, flat_terrain_contains, item_sprite_key, lighting};
use crate::{Assets, BBox, Map, Sector, Tile, Value, chunkbuilder::terrain_surface::TerrainSurface};
use scenevm::isometric::{ShadowGeometry, ShadowVertex};
use std::{collections::HashMap, sync::Arc};
use uuid::Uuid;
use vek::{Vec2, Vec3};
mod collect;
mod contact;
mod occlusion;
pub use contact::ContactPolicy;
mod source;
pub use source::{Sprite, SpriteImage, SpriteOwner, TerrainContact};

const SHADOW_CHUNK_SIZE: f32 = 16.;
const SHADOW_STORAGE_BYTES: usize = 64 * 1024 * 1024;

#[derive(PartialEq)]
struct TerrainShadowKey {
    map: Uuid,
    revision: u32,
    bounds: BBox,
    stride: usize,
}

pub struct ShadowView<'a> {
    pub projection: Projection,
    pub size: (u32, u32),
    pub lighting: &'a lighting::Lighting,
    pub max_storage_bytes: usize,
}

/// Geometry caches shared by native GPU production and the reference adapter.
/// No viewport-sized albedo, surface, depth or ownership array belongs here.
#[derive(Default)]
pub struct SceneCache {
    pub player_view: crate::party::PlayerView,
    terrain_key: Option<(Uuid, u32)>,
    terrain_model: Option<Arc<TerrainSurface>>,
    foreground_fade: occlusion::ForegroundFade,
    pub(super) shadow_geometry: Option<(u64, Arc<ShadowGeometry>)>,
    terrain_shadow: Option<(TerrainShadowKey, Arc<ShadowGeometry>)>,
}

/// A scene frame borrowing authored/derived asset sources in reference painter order.
/// Geometry and sprite bytes are source assets; none are a composited viewport.
pub struct ScenePlan<'a> {
    /// Tick-selected floor/terrain phase before spatial page addressing.
    pub material_frame: usize,
    pub terrain: Option<Arc<TerrainSurface>>,
    pub ground: Vec<Sprite<'a>>,
    pub upright: Vec<Sprite<'a>>,
    pub floors: Vec<(&'a Sector, &'a Tile)>,
    pub casters: Vec<lighting::Caster<'a>>,
    mesh_instances: Vec<(&'a ShadowGeometry, Vec3<f32>, f32)>,
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
        let terrain = self.terrain(map);
        collect::collect(
            map,
            assets,
            seconds,
            avatars,
            terrain,
            self.player_view,
            &mut self.foreground_fade,
        )
    }

    /// CPU scene inspection with a bounded geometry budget.
    pub fn shadow_geometry(
        &mut self,
        map: &Map,
        assets: &Assets,
        plan: &ScenePlan<'_>,
    ) -> Option<Arc<ShadowGeometry>> {
        self.build_shadow_geometry(map, assets, plan, None, SHADOW_STORAGE_BYTES)
    }

    /// Keep shadow work local to visible receivers and lights that can reach them.
    pub fn shadow_geometry_in_view(
        &mut self,
        map: &Map,
        assets: &Assets,
        plan: &ScenePlan<'_>,
        view: ShadowView<'_>,
    ) -> Option<Arc<ShadowGeometry>> {
        let mut heights = [0., plan.terrain.as_ref().map_or(0., |terrain| terrain.max_height)];
        for sprite in plan.ground.iter().chain(&plan.upright) {
            let span = sprite.image_info(false).map_or(0., |image| {
                image.height as f32 / image.unit.max(0.001) * sprite.scale.max(0.01)
            });
            heights[0] = heights[0].min(sprite.height - span);
            heights[1] = heights[1].max(sprite.height + span);
        }
        let receivers = view.projection.visible_bounds(view.size, heights);
        let bounds = view.lighting.shadow_bounds(receivers)?;
        let bounds = BBox::new(
            (bounds.min / SHADOW_CHUNK_SIZE).map(f32::floor) * SHADOW_CHUNK_SIZE,
            (bounds.max / SHADOW_CHUNK_SIZE).map(f32::ceil) * SHADOW_CHUNK_SIZE,
        );
        self.build_shadow_geometry(map, assets, plan, Some(bounds), view.max_storage_bytes)
    }

    fn build_shadow_geometry(
        &mut self,
        map: &Map,
        assets: &Assets,
        plan: &ScenePlan<'_>,
        bounds: Option<BBox>,
        max_storage_bytes: usize,
    ) -> Option<Arc<ShadowGeometry>> {
        use std::hash::{Hash, Hasher};
        // SceneVM rounds storage allocations to a power of two.
        let bytes = max_storage_bytes.min(SHADOW_STORAGE_BYTES);
        let bytes = (bytes + 1).next_power_of_two() / 2;
        let mut vertices_left = bytes / std::mem::size_of::<ShadowVertex>();
        let mut alpha_left = bytes / std::mem::size_of::<u32>();
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        (map.id, map.changed, assets.tiles_revision).hash(&mut hasher);
        let mut mesh_bounds = HashMap::new();
        let mut meshes = Vec::new();
        for &(mesh, pos, scale) in &plan.mesh_instances {
            let source = (mesh.vertices.as_ptr() as usize, mesh.vertices.len());
            if let Some(bounds) = bounds {
                let local = mesh_bounds.entry(source).or_insert_with(|| {
                    let mut min = Vec2::broadcast(f32::INFINITY);
                    let mut max = Vec2::broadcast(f32::NEG_INFINITY);
                    for vertex in &mesh.vertices {
                        let at = Vec2::new(vertex.position[0], vertex.position[2]);
                        min = min.map2(at, f32::min);
                        max = max.map2(at, f32::max);
                    }
                    BBox::new(min, max)
                });
                let offset = Vec2::new(pos.x, pos.z);
                let a = local.min * scale + offset;
                let b = local.max * scale + offset;
                if !bounds.intersects(&BBox::new(a.map2(b, f32::min), a.map2(b, f32::max))) {
                    continue;
                }
            }
            vertices_left = vertices_left.checked_sub(mesh.vertices.len())?;
            alpha_left = alpha_left.checked_sub(mesh.alpha.len())?;
            (source, pos.into_array().map(f32::to_bits), scale.to_bits()).hash(&mut hasher);
            meshes.push((mesh, pos, scale));
        }
        let terrain = plan.terrain.as_ref().and_then(|terrain| {
            let clip = bounds.unwrap_or(terrain.bounds);
            let clip = BBox::new(
                clip.min.map2(terrain.bounds.min, f32::max),
                clip.max.map2(terrain.bounds.max, f32::min),
            );
            let stride = terrain.stride_for_triangle_budget(clip, vertices_left / 3)?;
            (clip.min.into_array().map(f32::to_bits), clip.max.into_array().map(f32::to_bits), stride)
                .hash(&mut hasher);
            Some((terrain, TerrainShadowKey { map: map.id, revision: map.changed, bounds: clip, stride }))
        });
        let key = hasher.finish();
        let cached = self
            .shadow_geometry
            .as_ref()
            .filter(|c| c.0 == key)
            .map(|c| c.1.clone());
        let geometry = cached.unwrap_or_else(|| {
            let mut geometry = ShadowGeometry {
                revision: key,
                ..Default::default()
            };
            if let Some((terrain, terrain_key)) = terrain {
                let saved = self
                    .terrain_shadow
                    .as_ref()
                    .filter(|c| c.0 == terrain_key)
                    .map(|c| c.1.clone());
                let triangles = saved.unwrap_or_else(|| {
                    let mut triangles = ShadowGeometry::default();
                    terrain.triangles_with_stride(terrain_key.bounds, terrain_key.stride, |t| {
                        triangles.triangle(t.map(|p| [p.x, p.y, p.z]))
                    });
                    let triangles = std::sync::Arc::new(triangles);
                    self.terrain_shadow = Some((terrain_key, triangles.clone()));
                    triangles
                });
                geometry.append(&triangles, [0.; 3], 1.);
            }
            for (mesh, pos, scale) in meshes {
                geometry.append(mesh, pos.into_array(), scale);
            }
            let geometry = std::sync::Arc::new(geometry);
            self.shadow_geometry = Some((key, geometry.clone()));
            geometry
        });
        (!geometry.vertices.is_empty()).then_some(geometry)
    }
}
