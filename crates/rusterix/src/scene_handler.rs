use std::{hash::Hasher, str::FromStr};

use crate::{
    Assets, AvatarDirection, AvatarShadingOptions, BillboardAnimation, BillboardMetadata, D3Camera,
    Item, Map, ParticleEmitter, PixelSource, RenderSettings, Texture, Tile, Value,
    avatar_builder::AvatarRuntimeBuilder, chunkbuilder::d3chunkbuilder::DEFAULT_TILE_ID,
};
use indexmap::IndexMap;
use rust_embed::EmbeddedFile;
use rustc_hash::{FxHashMap, FxHashSet};
use scenevm::{Atom, Chunk, DynamicObject, GeoId, Light, SceneVM};
use theframework::prelude::*;

/// Tracks per-billboard animation state so we can interpolate on visibility changes.
#[derive(Default)]
pub(crate) struct BillboardAnimState {
    start_open: f32,
    target_open: f32,
    start_frame: usize,
}

#[derive(Clone, Copy)]
enum AnimationClock {
    Render,   // use render frames (~30 fps)
    GameTick, // use animation_frame ticks (~4 fps default)
}

impl BillboardAnimState {
    fn new(initial_open: f32, frame: usize) -> Self {
        Self {
            start_open: initial_open,
            target_open: initial_open,
            start_frame: frame,
        }
    }

    /// Returns interpolated open amount for the given frame using a smoothstep curve.
    fn open_amount(&self, frame: usize, fps: f32, duration_seconds: f32) -> f32 {
        if duration_seconds <= 0.0 {
            return self.target_open;
        }
        let elapsed_seconds = frame.saturating_sub(self.start_frame) as f32 / fps;
        let t = (elapsed_seconds / duration_seconds).clamp(0.0, 1.0);
        let smooth = t * t * (3.0 - 2.0 * t); // smoothstep
        self.start_open + (self.target_open - self.start_open) * smooth
    }
}

pub struct SceneHandler {
    pub vm: SceneVM,

    pub overlay_2d_id: Uuid,
    pub overlay_2d: Chunk,

    pub overlay_3d_id: Uuid,
    pub overlay_3d: Chunk,

    pub character_off: Uuid,
    pub character_on: Uuid,
    pub item_off: Uuid,
    pub item_on: Uuid,

    pub flat_material: Uuid,

    pub white: Uuid,
    pub selected: Uuid,
    pub gray: Uuid,
    pub outline: Uuid,
    pub yellow: Uuid,

    pub settings: RenderSettings,

    // Billboards for dynamic doors/gates (indexed by GeoId for fast lookup)
    pub billboards: FxHashMap<GeoId, BillboardMetadata>,

    // Animation state per billboard
    pub(crate) billboard_anim_states: FxHashMap<GeoId, BillboardAnimState>,
    // Per-item animation phase starts for one-shot impact effects.
    impact_anim_starts: FxHashMap<GeoId, u32>,
    campfire_emitters: FxHashMap<u32, ParticleEmitter>,
    tile_emitters_2d: FxHashMap<u32, ParticleEmitter>,
    tile_emitters_3d: FxHashMap<u32, ParticleEmitter>,

    // Local render-frame counter for timing animations at fixed FPS
    frame_counter: usize,
    avatar_builder: AvatarRuntimeBuilder,
    last_dynamics_hash_2d: Option<u64>,
    last_dynamics_hash_3d: Option<u64>,
    last_dynamics_tick_2d: Option<usize>,
    last_dynamics_tick_3d: Option<usize>,
    dynamics_ready_2d: bool,
    dynamics_ready_3d: bool,

    // Timing parameters (configurable)
    render_fps: f32,
    game_tick_fps: f32,
}

impl Default for SceneHandler {
    fn default() -> Self {
        SceneHandler::empty()
    }
}

impl SceneHandler {
    fn particle_sprite_tile_id(tile_id: Uuid) -> Uuid {
        Uuid::from_u128(tile_id.as_u128() ^ 0x705f_6172_7469_636c_655f_7370_7269)
    }

    fn build_particle_sprite_texture(color: [u8; 4]) -> Texture {
        let size = 32usize;
        let mut data = vec![0u8; size * size * 4];
        let center = (size as f32 - 1.0) * 0.5;
        let radius = center.max(1.0);

        for y in 0..size {
            for x in 0..size {
                let dx = (x as f32 - center) / radius;
                let dy = (y as f32 - center) / radius;
                let dist = (dx * dx + dy * dy).sqrt();
                let falloff = (1.0 - dist).clamp(0.0, 1.0);
                let alpha = (falloff * falloff * 255.0) as u8;
                let boost = 0.6 + falloff * 0.4;
                let idx = (y * size + x) * 4;
                data[idx] = ((color[0] as f32) * boost).clamp(0.0, 255.0) as u8;
                data[idx + 1] = ((color[1] as f32) * boost).clamp(0.0, 255.0) as u8;
                data[idx + 2] = ((color[2] as f32) * boost).clamp(0.0, 255.0) as u8;
                data[idx + 3] = alpha;
            }
        }

        Texture::new(data, size, size)
    }

    fn rebuild_campfire_particles(
        &mut self,
        map: &Map,
        camera: &dyn D3Camera,
        assets: &Assets,
    ) -> bool {
        let mut top_floor_y_by_sector: FxHashMap<u32, f32> = FxHashMap::default();
        for surface in map.surfaces.values() {
            if surface.plane.normal.y.abs() <= 0.7 {
                continue;
            }
            top_floor_y_by_sector
                .entry(surface.sector_id)
                .and_modify(|y| {
                    if surface.plane.origin.y > *y {
                        *y = surface.plane.origin.y;
                    }
                })
                .or_insert(surface.plane.origin.y);
        }

        let basis = camera.basis_vectors();
        let dt = (1.0 / self.render_fps.max(1.0)).clamp(0.005, 0.1);
        let mut active_emitters: FxHashSet<u32> = FxHashSet::default();
        let mut has_particles = false;

        for sector in &map.sectors {
            let feature = sector
                .properties
                .get_str_default("sector_feature", "None".to_string());
            if feature != "Campfire" {
                continue;
            }

            let flame_height = sector
                .properties
                .get_float_default("campfire_flame_height", 0.8)
                .max(0.0);
            let flame_width = sector
                .properties
                .get_float_default("campfire_flame_width", 0.45)
                .max(0.05);
            if flame_height <= 0.0 {
                continue;
            }

            let flame_tile_id = sector
                .properties
                .get_source("campfire_flame_source")
                .and_then(|source| source.tile_from_tile_list(assets))
                .map(|tile| Self::particle_sprite_tile_id(tile.id))
                .or_else(|| {
                    sector
                        .properties
                        .get_source("source")
                        .and_then(|source| source.tile_from_tile_list(assets))
                        .map(|tile| Self::particle_sprite_tile_id(tile.id))
                })
                .unwrap_or_else(|| Uuid::from_str(DEFAULT_TILE_ID).unwrap());

            let Some(center) = sector.center(map) else {
                continue;
            };
            let base_y = top_floor_y_by_sector
                .get(&sector.id)
                .copied()
                .unwrap_or(0.0);
            let origin = Vec3::new(center.x, base_y + 0.05, center.y);
            active_emitters.insert(sector.id);

            let emitter = self
                .campfire_emitters
                .entry(sector.id)
                .or_insert_with(|| ParticleEmitter::new(origin, Vec3::new(0.0, 1.0, 0.0)));
            emitter.origin = origin;
            emitter.direction = Vec3::new(0.0, 1.0, 0.0);
            emitter.spread = 0.75;
            emitter.rate = 28.0 + flame_height * 16.0;
            emitter.color = [255, 180, 90, 255];
            emitter.color_variation = 25;
            emitter.lifetime_range = (0.4, 0.95);
            emitter.radius_range = (
                (flame_width * 0.35).max(0.12),
                (flame_width * 0.85).max(0.28),
            );
            emitter.speed_range = (flame_height * 0.75, flame_height * 1.6);
            emitter.update(dt);

            for (index, particle) in emitter.particles.iter().enumerate() {
                has_particles = true;
                let opacity = (particle.lifetime / emitter.lifetime_range.1).clamp(0.0, 1.0);
                let size = (particle.radius * 2.8).max(0.08);
                let center = particle.pos + Vec3::new(0.0, size * 0.35, 0.0);
                let dynamic = DynamicObject::billboard_tile(
                    GeoId::Unknown(sector.id.saturating_mul(1024).saturating_add(index as u32)),
                    flame_tile_id,
                    center,
                    basis.1,
                    basis.2,
                    size,
                    size * 2.1,
                )
                .with_opacity(opacity);
                self.vm.execute(Atom::AddDynamic { object: dynamic });
            }
        }

        self.campfire_emitters
            .retain(|sector_id, _| active_emitters.contains(sector_id));
        has_particles
    }

    #[inline]
    fn tile_particle_key(kind: u32, host_id: u32, slot: u32) -> u32 {
        kind.wrapping_mul(0x1f1f_1f1f)
            ^ host_id.wrapping_mul(0x045d_9f3b)
            ^ slot.wrapping_mul(0x119d_e1f3)
    }

    fn rebuild_tile_particles_2d(&mut self, map: &Map, assets: &Assets) -> bool {
        let dt = (1.0 / self.render_fps.max(1.0)).clamp(0.005, 0.1);
        let mut active_emitters: FxHashSet<u32> = FxHashSet::default();
        let mut has_particles = false;

        let spawn_2d = |emitters: &mut FxHashMap<u32, ParticleEmitter>,
                        vm: &mut SceneVM,
                        key: u32,
                        tile: &Tile,
                        pos: Vec2<f32>,
                        size_scale: f32,
                        direction: Vec3<f32>,
                        layer: i32,
                        active_emitters: &mut FxHashSet<u32>,
                        has_particles: &mut bool| {
            let Some(def) = &tile.particle_emitter else {
                return;
            };
            active_emitters.insert(key);
            let emitter = emitters.entry(key).or_insert_with(|| {
                let mut emitter = def.clone();
                emitter.origin = Vec3::new(pos.x, pos.y, 0.0);
                emitter.direction = direction.normalized();
                emitter.time_accum = 0.0;
                emitter.particles.clear();
                emitter
            });
            emitter.origin = Vec3::new(pos.x, pos.y, 0.0);
            emitter.direction = direction.normalized();
            emitter.update(dt);

            let lifetime_max = emitter.lifetime_range.1.max(0.001);
            for (index, particle) in emitter.particles.iter().enumerate() {
                *has_particles = true;
                let opacity = (particle.lifetime / lifetime_max).clamp(0.0, 1.0);
                let size = (particle.radius * 2.2 * size_scale).max(0.08);
                let dynamic = DynamicObject::billboard_tile_2d(
                    GeoId::Unknown(key.wrapping_mul(1024).wrapping_add(index as u32)),
                    Self::particle_sprite_tile_id(tile.id),
                    Vec2::new(particle.pos.x, particle.pos.y),
                    size,
                    size,
                )
                .with_layer(layer)
                .with_opacity(opacity);
                vm.execute(Atom::AddDynamic { object: dynamic });
            }
        };

        for sector in &map.sectors {
            if let Some(center) = sector.center(map)
                && let Some(source) = sector.properties.get_default_source()
                && let Some(tile) = source.tile_from_tile_list(assets)
            {
                spawn_2d(
                    &mut self.tile_emitters_2d,
                    &mut self.vm,
                    Self::tile_particle_key(1, sector.id, 0),
                    &tile,
                    center,
                    1.0,
                    Vec3::new(0.0, -1.0, 0.0),
                    5,
                    &mut active_emitters,
                    &mut has_particles,
                );
            }
        }

        for linedef in &map.linedefs {
            if let Some(Value::Source(source)) = linedef.properties.get("row1_source")
                && let Some(tile) = source.tile_from_tile_list(assets)
                && let (Some(start), Some(end)) = (
                    map.find_vertex(linedef.start_vertex),
                    map.find_vertex(linedef.end_vertex),
                )
            {
                spawn_2d(
                    &mut self.tile_emitters_2d,
                    &mut self.vm,
                    Self::tile_particle_key(2, linedef.id, 0),
                    &tile,
                    Vec2::new((start.x + end.x) * 0.5, (start.y + end.y) * 0.5),
                    1.0,
                    Vec3::new(0.0, -1.0, 0.0),
                    6,
                    &mut active_emitters,
                    &mut has_particles,
                );
            }
        }

        for vertex in &map.vertices {
            if let Some(Value::Source(PixelSource::TileId(tile_id))) =
                vertex.properties.get("source")
                && let Some(tile) = assets.tiles.get(tile_id)
            {
                spawn_2d(
                    &mut self.tile_emitters_2d,
                    &mut self.vm,
                    Self::tile_particle_key(3, vertex.id, 0),
                    &tile,
                    Vec2::new(vertex.x, vertex.y),
                    vertex
                        .properties
                        .get_float_default("source_size", 1.0)
                        .max(0.25),
                    Vec3::new(0.0, -1.0, 0.0),
                    15,
                    &mut active_emitters,
                    &mut has_particles,
                );
            }
        }

        for item in &map.items {
            if item.attributes.get_bool_default("visible", false)
                && let Some(Value::Source(source)) = item.attributes.get("source")
                && let Some(tile) = source.tile_from_tile_list(assets)
            {
                spawn_2d(
                    &mut self.tile_emitters_2d,
                    &mut self.vm,
                    Self::tile_particle_key(4, item.id, 0),
                    &tile,
                    Vec2::new(item.position.x, item.position.z),
                    1.0,
                    Vec3::new(0.0, -1.0, 0.0),
                    25,
                    &mut active_emitters,
                    &mut has_particles,
                );
            }
        }

        for entity in &map.entities {
            if entity.attributes.get_bool_default("visible", false)
                && let Some(Value::Source(source)) = entity.attributes.get("source")
                && let Some(tile) = source.tile_from_tile_list(assets)
            {
                spawn_2d(
                    &mut self.tile_emitters_2d,
                    &mut self.vm,
                    Self::tile_particle_key(5, entity.id, 0),
                    &tile,
                    Vec2::new(entity.position.x, entity.position.z),
                    entity.attributes.get_float_default("size", 1.0).max(0.25),
                    Vec3::new(0.0, -1.0, 0.0),
                    30,
                    &mut active_emitters,
                    &mut has_particles,
                );
            }
        }

        self.tile_emitters_2d
            .retain(|key, _| active_emitters.contains(key));
        has_particles
    }

    fn rebuild_tile_particles_3d(
        &mut self,
        map: &Map,
        camera: &dyn D3Camera,
        assets: &Assets,
    ) -> bool {
        let mut top_floor_y_by_sector: FxHashMap<u32, f32> = FxHashMap::default();
        for surface in map.surfaces.values() {
            if surface.plane.normal.y.abs() <= 0.7 {
                continue;
            }
            top_floor_y_by_sector
                .entry(surface.sector_id)
                .and_modify(|y| {
                    if surface.plane.origin.y > *y {
                        *y = surface.plane.origin.y;
                    }
                })
                .or_insert(surface.plane.origin.y);
        }

        let basis = camera.basis_vectors();
        let dt = (1.0 / self.render_fps.max(1.0)).clamp(0.005, 0.1);
        let mut active_emitters: FxHashSet<u32> = FxHashSet::default();
        let mut has_particles = false;

        let spawn_3d = |emitters: &mut FxHashMap<u32, ParticleEmitter>,
                        vm: &mut SceneVM,
                        key: u32,
                        tile: &Tile,
                        origin: Vec3<f32>,
                        size_scale: f32,
                        direction: Vec3<f32>,
                        active_emitters: &mut FxHashSet<u32>,
                        has_particles: &mut bool| {
            let Some(def) = &tile.particle_emitter else {
                return;
            };
            active_emitters.insert(key);
            let emitter = emitters.entry(key).or_insert_with(|| {
                let mut emitter = def.clone();
                emitter.origin = origin;
                emitter.direction = direction.normalized();
                emitter.time_accum = 0.0;
                emitter.particles.clear();
                emitter
            });
            emitter.origin = origin;
            emitter.direction = direction.normalized();
            emitter.update(dt);

            let lifetime_max = emitter.lifetime_range.1.max(0.001);
            for (index, particle) in emitter.particles.iter().enumerate() {
                *has_particles = true;
                let opacity = (particle.lifetime / lifetime_max).clamp(0.0, 1.0);
                let size = (particle.radius * 2.4 * size_scale).max(0.08);
                let center = particle.pos + Vec3::new(0.0, size * 0.2, 0.0);
                let dynamic = DynamicObject::billboard_tile(
                    GeoId::Unknown(key.wrapping_mul(1024).wrapping_add(index as u32)),
                    Self::particle_sprite_tile_id(tile.id),
                    center,
                    basis.1,
                    basis.2,
                    size,
                    size,
                )
                .with_opacity(opacity);
                vm.execute(Atom::AddDynamic { object: dynamic });
            }
        };

        for sector in &map.sectors {
            if let Some(center) = sector.center(map) {
                let floor_y = top_floor_y_by_sector
                    .get(&sector.id)
                    .copied()
                    .unwrap_or_else(|| {
                        crate::chunkbuilder::terrain_generator::TerrainGenerator::sample_height_at(
                            map,
                            center,
                            &crate::chunkbuilder::terrain_generator::TerrainConfig::default(),
                        )
                    });

                if let Some(source) = sector.properties.get_default_source()
                    && let Some(tile) = source.tile_from_tile_list(assets)
                {
                    spawn_3d(
                        &mut self.tile_emitters_3d,
                        &mut self.vm,
                        Self::tile_particle_key(1, sector.id, 0),
                        &tile,
                        Vec3::new(center.x, floor_y + 0.02, center.y),
                        1.0,
                        Vec3::new(0.0, 1.0, 0.0),
                        &mut active_emitters,
                        &mut has_particles,
                    );
                }

                if let Some(source) = sector.properties.get_source("ceiling_source")
                    && let Some(tile) = source.tile_from_tile_list(assets)
                {
                    spawn_3d(
                        &mut self.tile_emitters_3d,
                        &mut self.vm,
                        Self::tile_particle_key(1, sector.id, 1),
                        &tile,
                        Vec3::new(
                            center.x,
                            sector
                                .properties
                                .get_float_default("ceiling_height", floor_y + 1.0)
                                - 0.02,
                            center.y,
                        ),
                        1.0,
                        Vec3::new(0.0, -1.0, 0.0),
                        &mut active_emitters,
                        &mut has_particles,
                    );
                }
            }
        }

        for linedef in &map.linedefs {
            if let (Some(start), Some(end)) = (
                map.find_vertex(linedef.start_vertex),
                map.find_vertex(linedef.end_vertex),
            ) {
                let start3 = start.as_vec3_world();
                let end3 = end.as_vec3_world();
                let span = end3 - start3;
                let span_len = span.magnitude().max(0.001);
                let along = (end3 - start3)
                    .try_normalized()
                    .unwrap_or(Vec3::new(1.0, 0.0, 0.0));
                let outward = Vec3::new(-along.z, 0.0, along.x)
                    .try_normalized()
                    .unwrap_or(Vec3::new(0.0, 0.0, 1.0));
                let base_y_from_sector = linedef
                    .sector_ids
                    .iter()
                    .filter_map(|sector_id| map.find_sector(*sector_id))
                    .map(|sector| sector.properties.get_float_default("floor_height", 0.0))
                    .reduce(f32::min);
                let base_y = base_y_from_sector.unwrap_or(start3.y.min(end3.y));
                let wall_height = linedef
                    .properties
                    .get_float_default("wall_height", 2.0)
                    .max(0.25);
                let active_rows: Vec<(&str, u32)> =
                    ["row1_source", "row2_source", "row3_source", "row4_source"]
                        .into_iter()
                        .enumerate()
                        .filter_map(|(i, key)| {
                            matches!(linedef.properties.get(key), Some(Value::Source(_)))
                                .then_some((key, i as u32))
                        })
                        .collect();
                let active_count = active_rows.len().max(1) as f32;
                let emitter_count = ((span_len / 0.8).ceil() as usize).clamp(2, 8);
                for (ordinal, (source_key, slot)) in active_rows.iter().enumerate() {
                    if let Some(Value::Source(source)) = linedef.properties.get(*source_key)
                        && let Some(tile) = source.tile_from_tile_list(assets)
                    {
                        let band_t = (ordinal as f32 + 0.5) / active_count;
                        for emitter_index in 0..emitter_count {
                            let across_t = if emitter_count == 1 {
                                0.5
                            } else {
                                (emitter_index as f32 + 0.5) / emitter_count as f32
                            };
                            let horizontal_jitter =
                                ((ordinal as f32 * 13.0 + emitter_index as f32 * 7.0).sin())
                                    * (0.35 / emitter_count as f32);
                            let vertical_jitter =
                                ((ordinal as f32 * 11.0 + emitter_index as f32 * 5.0).cos())
                                    * 0.06
                                    * wall_height;
                            let t = (across_t + horizontal_jitter).clamp(0.05, 0.95);
                            let origin = start3
                                + span * t
                                + Vec3::new(
                                    0.0,
                                    base_y + wall_height * band_t - start3.y + vertical_jitter,
                                    0.0,
                                )
                                + outward * 0.12;
                            let direction = (Vec3::new(0.0, 1.0, 0.0) + outward * 0.18)
                                .try_normalized()
                                .unwrap_or(Vec3::new(0.0, 1.0, 0.0));
                            spawn_3d(
                                &mut self.tile_emitters_3d,
                                &mut self.vm,
                                Self::tile_particle_key(
                                    2,
                                    linedef.id,
                                    slot.wrapping_mul(16).wrapping_add(emitter_index as u32),
                                ),
                                &tile,
                                origin,
                                2.8,
                                direction,
                                &mut active_emitters,
                                &mut has_particles,
                            );
                        }
                    }
                }
            }
        }

        for vertex in &map.vertices {
            if let Some(Value::Source(PixelSource::TileId(tile_id))) =
                vertex.properties.get("source")
                && let Some(tile) = assets.tiles.get(tile_id)
            {
                spawn_3d(
                    &mut self.tile_emitters_3d,
                    &mut self.vm,
                    Self::tile_particle_key(3, vertex.id, 0),
                    &tile,
                    vertex.as_vec3_world(),
                    vertex
                        .properties
                        .get_float_default("source_size", 1.0)
                        .max(0.25),
                    Vec3::new(0.0, 1.0, 0.0),
                    &mut active_emitters,
                    &mut has_particles,
                );
            }
        }

        for item in &map.items {
            if item.attributes.get_bool_default("visible", false)
                && let Some(Value::Source(source)) = item.attributes.get("source")
                && let Some(tile) = source.tile_from_tile_list(assets)
            {
                spawn_3d(
                    &mut self.tile_emitters_3d,
                    &mut self.vm,
                    Self::tile_particle_key(4, item.id, 0),
                    &tile,
                    item.position,
                    1.0,
                    Vec3::new(0.0, 1.0, 0.0),
                    &mut active_emitters,
                    &mut has_particles,
                );
            }
        }

        for entity in &map.entities {
            if entity.attributes.get_bool_default("visible", false)
                && let Some(Value::Source(source)) = entity.attributes.get("source")
                && let Some(tile) = source.tile_from_tile_list(assets)
            {
                spawn_3d(
                    &mut self.tile_emitters_3d,
                    &mut self.vm,
                    Self::tile_particle_key(5, entity.id, 0),
                    &tile,
                    entity.position,
                    entity.attributes.get_float_default("size", 1.0).max(0.25),
                    Vec3::new(0.0, 1.0, 0.0),
                    &mut active_emitters,
                    &mut has_particles,
                );
            }
        }

        self.tile_emitters_3d
            .retain(|key, _| active_emitters.contains(key));
        has_particles
    }

    #[inline]
    fn impact_anim_start_for_item(&mut self, geo_id: GeoId, item: &Item) -> Option<u32> {
        if item.attributes.get_bool_default("spell_impacting", false) {
            let frame = self.frame_counter as u32;
            let start = self.impact_anim_starts.entry(geo_id).or_insert(frame);
            Some(*start)
        } else {
            self.impact_anim_starts.remove(&geo_id);
            None
        }
    }

    /// Invalidate dynamic entity/item/light caches so next frame rebuilds overlays.
    pub fn mark_dynamics_dirty(&mut self) {
        self.last_dynamics_hash_2d = None;
        self.last_dynamics_hash_3d = None;
        self.last_dynamics_tick_2d = None;
        self.last_dynamics_tick_3d = None;
        self.dynamics_ready_2d = false;
        self.dynamics_ready_3d = false;
    }

    /// Clear cached runtime geometry, billboards, lights and dynamic state.
    pub fn clear_runtime_scene(&mut self) {
        self.vm.execute(Atom::ClearGeometry);
        self.vm.execute(Atom::ClearDynamics);
        self.vm.execute(Atom::ClearLights);
        self.billboards.clear();
        self.billboard_anim_states.clear();
        self.impact_anim_starts.clear();
        self.campfire_emitters.clear();
        self.tile_emitters_2d.clear();
        self.tile_emitters_3d.clear();
        self.mark_dynamics_dirty();
    }

    /// Clear only runtime overlays and dynamic caches, keeping base geometry intact.
    pub fn clear_runtime_overlays(&mut self) {
        self.vm.execute(Atom::ClearDynamics);
        self.vm.execute(Atom::ClearLights);
        self.billboards.clear();
        self.billboard_anim_states.clear();
        self.impact_anim_starts.clear();
        self.campfire_emitters.clear();
        self.tile_emitters_2d.clear();
        self.tile_emitters_3d.clear();
        self.mark_dynamics_dirty();
    }

    pub fn add_sector_campfire_lights(&mut self, map: &Map) {
        let mut top_floor_y_by_sector: FxHashMap<u32, f32> = FxHashMap::default();
        for surface in map.surfaces.values() {
            if surface.plane.normal.y.abs() <= 0.7 {
                continue;
            }
            top_floor_y_by_sector
                .entry(surface.sector_id)
                .and_modify(|y| {
                    if surface.plane.origin.y > *y {
                        *y = surface.plane.origin.y;
                    }
                })
                .or_insert(surface.plane.origin.y);
        }

        for sector in &map.sectors {
            let feature = sector
                .properties
                .get_str_default("sector_feature", "None".to_string());
            if feature != "Campfire" {
                continue;
            }

            let intensity = sector
                .properties
                .get_float_default("campfire_light_intensity", 2.2)
                .max(0.0);
            let range = sector
                .properties
                .get_float_default("campfire_light_range", 5.0)
                .max(0.0);
            if intensity <= 0.0 || range <= 0.0 {
                continue;
            }
            let flicker = sector
                .properties
                .get_float_default("campfire_light_flicker", 0.2)
                .clamp(0.0, 1.0);
            let lift = sector
                .properties
                .get_float_default("campfire_light_lift", 0.2)
                .max(0.0);
            let flame_height = sector
                .properties
                .get_float_default("campfire_flame_height", 0.8)
                .max(0.0);

            let Some(center) = sector.center(map) else {
                continue;
            };
            let base_y = top_floor_y_by_sector
                .get(&sector.id)
                .copied()
                .unwrap_or(0.0);
            let flame_base_y = base_y + 0.02;
            let flame_center_y = flame_base_y + flame_height * 0.5;
            let position = Vec3::new(center.x, flame_center_y + lift, center.y);

            self.vm.execute(Atom::AddLight {
                id: GeoId::Sector(sector.id),
                light: Light::new_pointlight(position)
                    .with_color(Vec3::new(1.0, 0.62, 0.28))
                    .with_intensity(intensity)
                    .with_emitting(true)
                    .with_start_distance(0.0)
                    .with_end_distance(range)
                    .with_flicker(flicker),
            });
        }
    }

    pub fn find_item_any<'m>(map: &'m Map, id: u32) -> Option<&'m Item> {
        if let Some(item) = map.items.iter().find(|i| i.id == id) {
            return Some(item);
        }
        for profile_map in map.profiles.values() {
            if let Some(item) = profile_map.items.iter().find(|i| i.id == id) {
                return Some(item);
            }
        }
        None
    }

    pub fn find_item_by_profile_attrs<'m>(
        map: &'m Map,
        host_sector: Option<u32>,
        profile_sector: Option<u32>,
    ) -> Option<&'m Item> {
        let Some(host) = host_sector else { return None };
        let Some(profile) = profile_sector else {
            return None;
        };

        map.items.iter().find(|item| {
            let host_matches = match item.attributes.get("profile_host_sector_id") {
                Some(Value::UInt(v)) => *v == host,
                _ => false,
            };
            let profile_matches = match item.attributes.get("profile_sector_id") {
                Some(Value::UInt(v)) => *v == profile,
                _ => false,
            };
            host_matches && profile_matches
        })
    }

    pub fn empty() -> Self {
        let mut vm = SceneVM::default();
        vm.set_layer_activity_logging(false);

        Self {
            vm,

            overlay_2d_id: Uuid::new_v4(),
            overlay_2d: Chunk::default(),

            overlay_3d_id: Uuid::new_v4(),
            overlay_3d: Chunk::default(),

            character_off: Uuid::new_v4(),
            character_on: Uuid::new_v4(),
            item_off: Uuid::new_v4(),
            item_on: Uuid::new_v4(),

            flat_material: Uuid::new_v4(),

            white: Uuid::new_v4(),
            selected: Uuid::new_v4(),
            gray: Uuid::new_v4(),
            outline: Uuid::new_v4(),
            yellow: Uuid::new_v4(),

            settings: RenderSettings::default(),

            billboards: FxHashMap::default(),
            billboard_anim_states: FxHashMap::default(),
            impact_anim_starts: FxHashMap::default(),
            campfire_emitters: FxHashMap::default(),
            tile_emitters_2d: FxHashMap::default(),
            tile_emitters_3d: FxHashMap::default(),
            frame_counter: 0,
            avatar_builder: AvatarRuntimeBuilder::default(),
            last_dynamics_hash_2d: None,
            last_dynamics_hash_3d: None,
            last_dynamics_tick_2d: None,
            last_dynamics_tick_3d: None,
            dynamics_ready_2d: false,
            dynamics_ready_3d: false,
            render_fps: 30.0,
            game_tick_fps: 4.0, // default 250ms ticks
        }
    }

    #[inline]
    fn hash_vec3(hasher: &mut rustc_hash::FxHasher, v: Vec3<f32>) {
        hasher.write_u32(v.x.to_bits());
        hasher.write_u32(v.y.to_bits());
        hasher.write_u32(v.z.to_bits());
    }

    #[inline]
    fn avatar_direction_3d(entity: &crate::Entity, camera: &dyn D3Camera) -> AvatarDirection {
        let mut forward = entity.orientation;
        if forward.magnitude_squared() <= 1e-6 {
            return AvatarDirection::Front;
        }
        forward = forward.normalized();

        let to_camera = Vec2::new(
            camera.position().x - entity.position.x,
            camera.position().z - entity.position.z,
        );
        if to_camera.magnitude_squared() <= 1e-6 {
            return AvatarDirection::Front;
        }
        let to_camera = to_camera.normalized();

        // Right vector in XZ for forward=(x,z) stored as Vec2(x,y).
        // Use the right-handed perpendicular so facing north maps right->+X.
        let right = Vec2::new(-forward.y, forward.x);
        let front_dot = forward.dot(to_camera);
        let right_dot = right.dot(to_camera);

        if front_dot.abs() >= right_dot.abs() {
            if front_dot >= 0.0 {
                AvatarDirection::Front
            } else {
                AvatarDirection::Back
            }
        } else if right_dot >= 0.0 {
            AvatarDirection::Right
        } else {
            AvatarDirection::Left
        }
    }

    #[inline]
    fn avatar_direction_from_orientation(entity: &crate::Entity) -> AvatarDirection {
        let dir = entity.orientation;
        if dir.x.abs() >= dir.y.abs() {
            if dir.x >= 0.0 {
                AvatarDirection::Right
            } else {
                AvatarDirection::Left
            }
        } else if dir.y >= 0.0 {
            AvatarDirection::Front
        } else {
            AvatarDirection::Back
        }
    }

    fn hash_pixel_source(hasher: &mut rustc_hash::FxHasher, source: &PixelSource) {
        match source {
            PixelSource::Off => hasher.write_u8(0),
            PixelSource::TileId(id) => {
                hasher.write_u8(1);
                hasher.write(id.as_bytes());
            }
            PixelSource::TileGroup(id) => {
                hasher.write_u8(13);
                hasher.write(id.as_bytes());
            }
            PixelSource::TileGroupMember {
                group_id,
                member_index,
            } => {
                hasher.write_u8(14);
                hasher.write(group_id.as_bytes());
                hasher.write_u16(*member_index);
            }
            PixelSource::ProceduralTile(id) => {
                hasher.write_u8(15);
                hasher.write(id.as_bytes());
            }
            PixelSource::PaletteIndex(i) => {
                hasher.write_u8(12);
                hasher.write_u16(*i);
            }
            PixelSource::MaterialId(id) => {
                hasher.write_u8(2);
                hasher.write(id.as_bytes());
            }
            PixelSource::Sequence(seq) => {
                hasher.write_u8(3);
                hasher.write(seq.as_bytes());
            }
            PixelSource::EntityTile(a, b) => {
                hasher.write_u8(4);
                hasher.write_u32(*a);
                hasher.write_u32(*b);
            }
            PixelSource::ItemTile(a, b) => {
                hasher.write_u8(5);
                hasher.write_u32(*a);
                hasher.write_u32(*b);
            }
            PixelSource::Color(c) => {
                hasher.write_u8(6);
                hasher.write_u32(c.r.to_bits());
                hasher.write_u32(c.g.to_bits());
                hasher.write_u32(c.b.to_bits());
                hasher.write_u32(c.a.to_bits());
            }
            PixelSource::ShapeFXGraphId(id) => {
                hasher.write_u8(7);
                hasher.write(id.as_bytes());
            }
            PixelSource::StaticTileIndex(i) => {
                hasher.write_u8(8);
                hasher.write_u16(*i);
            }
            PixelSource::DynamicTileIndex(i) => {
                hasher.write_u8(9);
                hasher.write_u16(*i);
            }
            PixelSource::Pixel(px) => {
                hasher.write_u8(10);
                hasher.write_u8(px[0]);
                hasher.write_u8(px[1]);
                hasher.write_u8(px[2]);
                hasher.write_u8(px[3]);
            }
            PixelSource::Terrain => hasher.write_u8(11),
        }
    }

    fn hash_light_value(hasher: &mut rustc_hash::FxHasher, light: &crate::Light) {
        let color = light.get_color();
        hasher.write_u32(color[0].to_bits());
        hasher.write_u32(color[1].to_bits());
        hasher.write_u32(color[2].to_bits());
        hasher.write_u32(light.get_intensity().to_bits());
        hasher.write_u32(light.get_start_distance().to_bits());
        hasher.write_u32(light.get_end_distance().to_bits());
        hasher.write_u32(light.get_flicker().to_bits());
        hasher.write_u8(u8::from(light.active));
    }

    fn dynamics_hash_2d(&self, map: &Map, animation_frame: usize) -> u64 {
        let mut hasher = rustc_hash::FxHasher::default();
        hasher.write(map.id.as_bytes());
        hasher.write_u64(animation_frame as u64);
        hasher.write_u64(map.sectors.len() as u64);
        hasher.write_u64(map.linedefs.len() as u64);
        hasher.write_u64(map.vertices.len() as u64);
        hasher.write_u64(map.items.len() as u64);
        hasher.write_u64(map.entities.len() as u64);

        for sector in &map.sectors {
            hasher.write_u32(sector.id);
            if let Some(source) = sector.properties.get_default_source() {
                Self::hash_pixel_source(&mut hasher, source);
            } else {
                hasher.write_u8(0);
            }
        }

        for linedef in &map.linedefs {
            hasher.write_u32(linedef.id);
            if let Some(Value::Source(source)) = linedef.properties.get("row1_source") {
                Self::hash_pixel_source(&mut hasher, source);
            } else {
                hasher.write_u8(0);
            }
        }

        for vertex in &map.vertices {
            hasher.write_u32(vertex.id);
            hasher.write_u32(vertex.x.to_bits());
            hasher.write_u32(vertex.y.to_bits());
            hasher.write_u32(vertex.z.to_bits());
            if let Some(Value::Source(source)) = vertex.properties.get("source") {
                Self::hash_pixel_source(&mut hasher, source);
            } else {
                hasher.write_u8(0);
            }
        }

        for item in &map.items {
            hasher.write_u32(item.id);
            hasher.write_u32(item.position.x.to_bits());
            hasher.write_u32(item.position.y.to_bits());
            hasher.write_u32(item.position.z.to_bits());
            hasher.write_u8(u8::from(item.attributes.get_bool_default("visible", false)));
            if let Some(Value::Source(source)) = item.attributes.get("source") {
                Self::hash_pixel_source(&mut hasher, source);
            } else {
                hasher.write_u8(0);
            }
            if let Some(Value::Light(light)) = item.attributes.get("light") {
                hasher.write_u8(1);
                Self::hash_light_value(&mut hasher, light);
            } else {
                hasher.write_u8(0);
            }
        }

        for entity in &map.entities {
            hasher.write_u32(entity.id);
            hasher.write_u32(entity.position.x.to_bits());
            hasher.write_u32(entity.position.y.to_bits());
            hasher.write_u32(entity.position.z.to_bits());
            hasher.write_u8(u8::from(
                entity.attributes.get_bool_default("visible", false),
            ));
            if let Some(Value::Source(source)) = entity.attributes.get("source") {
                Self::hash_pixel_source(&mut hasher, source);
            } else {
                hasher.write_u8(0);
            }
            if let Some(Value::Light(light)) = entity.attributes.get("light") {
                hasher.write_u8(1);
                Self::hash_light_value(&mut hasher, light);
            } else {
                hasher.write_u8(0);
            }
            // Avatar selection can change with attrs without position changes.
            hasher.write(
                entity
                    .get_attr_string("anim")
                    .unwrap_or_default()
                    .as_bytes(),
            );
            hasher.write(
                entity
                    .get_attr_string("perspective")
                    .unwrap_or_default()
                    .as_bytes(),
            );
        }

        hasher.finish()
    }

    fn dynamics_hash_3d(&self, map: &Map, camera: &dyn D3Camera, animation_frame: usize) -> u64 {
        let mut hasher = rustc_hash::FxHasher::default();
        hasher.write(map.id.as_bytes());
        hasher.write_u64(animation_frame as u64);
        hasher.write(camera.id().as_bytes());
        Self::hash_vec3(&mut hasher, camera.position());
        let (fwd, right, up) = camera.basis_vectors();
        Self::hash_vec3(&mut hasher, fwd);
        Self::hash_vec3(&mut hasher, right);
        Self::hash_vec3(&mut hasher, up);

        hasher.write_u64(map.sectors.len() as u64);
        hasher.write_u64(map.linedefs.len() as u64);
        hasher.write_u64(map.vertices.len() as u64);
        hasher.write_u64(map.items.len() as u64);
        hasher.write_u64(map.entities.len() as u64);

        for sector in &map.sectors {
            hasher.write_u32(sector.id);
            if let Some(source) = sector.properties.get_default_source() {
                Self::hash_pixel_source(&mut hasher, source);
            } else {
                hasher.write_u8(0);
            }
            if let Some(source) = sector.properties.get_source("ceiling_source") {
                Self::hash_pixel_source(&mut hasher, source);
            } else {
                hasher.write_u8(0);
            }
        }

        for linedef in &map.linedefs {
            hasher.write_u32(linedef.id);
            for key in ["row1_source", "row2_source", "row3_source"] {
                if let Some(Value::Source(source)) = linedef.properties.get(key) {
                    Self::hash_pixel_source(&mut hasher, source);
                } else {
                    hasher.write_u8(0);
                }
            }
        }

        for vertex in &map.vertices {
            hasher.write_u32(vertex.id);
            hasher.write_u32(vertex.x.to_bits());
            hasher.write_u32(vertex.y.to_bits());
            hasher.write_u32(vertex.z.to_bits());
            if let Some(Value::Source(source)) = vertex.properties.get("source") {
                Self::hash_pixel_source(&mut hasher, source);
            } else {
                hasher.write_u8(0);
            }
        }

        for item in &map.items {
            hasher.write_u32(item.id);
            hasher.write_u32(item.position.x.to_bits());
            hasher.write_u32(item.position.y.to_bits());
            hasher.write_u32(item.position.z.to_bits());
            hasher.write_u8(u8::from(item.attributes.get_bool_default("visible", false)));
            if let Some(Value::Source(source)) = item.attributes.get("source") {
                Self::hash_pixel_source(&mut hasher, source);
            } else {
                hasher.write_u8(0);
            }
            if let Some(Value::Light(light)) = item.attributes.get("light") {
                hasher.write_u8(1);
                Self::hash_light_value(&mut hasher, light);
            } else {
                hasher.write_u8(0);
            }
        }

        for entity in &map.entities {
            hasher.write_u32(entity.id);
            hasher.write_u32(entity.position.x.to_bits());
            hasher.write_u32(entity.position.y.to_bits());
            hasher.write_u32(entity.position.z.to_bits());
            hasher.write_u8(u8::from(
                entity.attributes.get_bool_default("visible", false),
            ));
            if let Some(Value::Source(source)) = entity.attributes.get("source") {
                Self::hash_pixel_source(&mut hasher, source);
            } else {
                hasher.write_u8(0);
            }
            if let Some(Value::Light(light)) = entity.attributes.get("light") {
                hasher.write_u8(1);
                Self::hash_light_value(&mut hasher, light);
            } else {
                hasher.write_u8(0);
            }
            hasher.write(
                entity
                    .get_attr_string("anim")
                    .unwrap_or_default()
                    .as_bytes(),
            );
            hasher.write(
                entity
                    .get_attr_string("perspective")
                    .unwrap_or_default()
                    .as_bytes(),
            );
        }

        hasher.finish()
    }

    pub fn set_timings(&mut self, render_fps: f32, game_tick_ms: i32) {
        self.render_fps = render_fps.max(1.0);
        if game_tick_ms > 0 {
            self.game_tick_fps = 1000.0 / game_tick_ms as f32;
        }
    }

    pub fn build_atlas(&mut self, tiles: &IndexMap<Uuid, Tile>, editor: bool) {
        for (id, tile) in tiles {
            let mut b = vec![];
            for t in &tile.textures {
                b.push(t.data.to_vec());
            }
            self.vm.execute(Atom::AddTile {
                id: *id,
                width: tile.textures[0].width as u32,
                height: tile.textures[0].height as u32,
                frames: tile.to_buffer_array(),
                material_frames: Some(tile.to_material_array()),
            });

            if let Some(emitter) = &tile.particle_emitter {
                let sprite = Self::build_particle_sprite_texture(emitter.color);
                self.vm.execute(Atom::AddTile {
                    id: Self::particle_sprite_tile_id(*id),
                    width: sprite.width as u32,
                    height: sprite.height as u32,
                    frames: vec![sprite.data],
                    material_frames: None,
                });
            }
        }

        if editor {
            fn decode_png(file: EmbeddedFile) -> Option<(Vec<u8>, u32, u32)> {
                // Use the `image` crate to decode, auto-detecting the format from bytes.
                match image::load_from_memory(&file.data) {
                    Ok(dynamic) => {
                        let rgba = dynamic.to_rgba8();
                        let (w, h) = rgba.dimensions();
                        Some((rgba.into_raw(), w, h))
                    }
                    Err(_) => None,
                }
            }

            if let Some(bytes) = crate::Embedded::get("icons/character_off.png") {
                if let Some((bytes, width, height)) = decode_png(bytes) {
                    self.vm.execute(Atom::AddTile {
                        id: self.character_off,
                        width,
                        height,
                        frames: vec![bytes],
                        material_frames: None,
                    });
                }
            }
            if let Some(bytes) = crate::Embedded::get("icons/character_on.png") {
                if let Some((bytes, width, height)) = decode_png(bytes) {
                    self.vm.execute(Atom::AddTile {
                        id: self.character_on,
                        width,
                        height,
                        frames: vec![bytes],
                        material_frames: None,
                    });
                }
            }
            if let Some(bytes) = crate::Embedded::get("icons/treasure_off.png") {
                if let Some((bytes, width, height)) = decode_png(bytes) {
                    self.vm.execute(Atom::AddTile {
                        id: self.item_off,
                        width,
                        height,
                        frames: vec![bytes],
                        material_frames: None,
                    });
                }
            }
            if let Some(bytes) = crate::Embedded::get("icons/treasure_on.png") {
                if let Some((bytes, width, height)) = decode_png(bytes) {
                    self.vm.execute(Atom::AddTile {
                        id: self.item_on,
                        width,
                        height,
                        frames: vec![bytes],
                        material_frames: None,
                    });
                }
            }
            let checker = Texture::checkerboard(100, 50);
            self.vm.execute(Atom::AddTile {
                id: Uuid::from_str("27826750-a9e7-4346-994b-fb318b238452")
                    .ok()
                    .unwrap(),
                width: 100,
                height: 100,
                frames: vec![checker.data],
                material_frames: None,
            });
            // self.vm.execute(Atom::AddSolid {
            //     id: Uuid::from_str("27826750-a9e7-4346-994b-fb318b238452")
            //         .ok()
            //         .unwrap(),
            //     color: [128, 128, 18, 255],
            // });
            self.vm.execute(Atom::AddSolid {
                id: self.white,
                color: [255, 255, 255, 255],
            });
            self.vm.execute(Atom::AddSolid {
                id: self.selected,
                color: [187, 122, 208, 255],
            });
            self.vm.execute(Atom::AddSolid {
                id: self.outline,
                color: [122, 208, 187, 255],
            });
            self.vm.execute(Atom::AddSolid {
                id: self.yellow,
                color: vek::Rgba::yellow().into_array(),
            });
        }

        self.vm.execute(Atom::BuildAtlas);
    }

    pub fn clear_overlay(&mut self) {
        if self.vm.vm_layer_count() == 1 {
            // 2D Overlay layer
            let idx = self.vm.add_vm_layer();
            self.vm.set_active_vm(idx);

            self.vm.execute(scenevm::Atom::SetBackground(Vec4::zero()));
            if let Some(bytes) = crate::Embedded::get("shader/2d_overlay_shader.wgsl") {
                if let Ok(source) = std::str::from_utf8(bytes.data.as_ref()) {
                    self.vm.execute(Atom::SetSource2D(source.into()));
                }
            }
            self.vm
                .execute(scenevm::Atom::SetRenderMode(scenevm::RenderMode::Compute2D));

            // 3D Overlay layer
            let idx = self.vm.add_vm_layer();
            self.vm.set_active_vm(idx);

            self.vm.execute(scenevm::Atom::SetBackground(Vec4::zero()));
            if let Some(bytes) = crate::Embedded::get("shader/3d_overlay_shader.wgsl") {
                if let Ok(source) = std::str::from_utf8(bytes.data.as_ref()) {
                    self.vm.execute(Atom::SetSource3D(source.into()));
                }
            }
            self.vm
                .execute(scenevm::Atom::SetRenderMode(scenevm::RenderMode::Raster3D));
        }
        self.vm.set_active_vm(0);

        self.overlay_2d = Chunk::default();
        self.overlay_2d.priority = 0;

        self.overlay_3d = Chunk::default();
        self.overlay_3d.priority = 0;
    }

    pub fn set_overlay(&mut self) {
        self.vm.set_active_vm(1);
        self.vm.execute(Atom::AddChunk {
            id: self.overlay_2d_id,
            chunk: self.overlay_2d.clone(),
        });
        self.vm.set_active_vm(2);
        self.vm.execute(Atom::AddChunk {
            id: self.overlay_3d_id,
            chunk: self.overlay_3d.clone(),
        });
        self.vm.set_active_vm(0);
    }

    pub fn add_overlay_2d_line(
        &mut self,
        id: GeoId,
        start: Vec2<f32>,
        end: Vec2<f32>,
        color: Uuid,
        layer: i32,
    ) {
        self.overlay_2d.add_line_strip_2d_px(
            id,
            color,
            vec![start.into_array(), end.into_array()],
            1.5,
            layer,
        );
    }

    /// Build dynamic elements of the 2D Map: Entities, Items, Lights ...
    pub fn build_dynamics_2d(&mut self, map: &Map, animation_frame: usize, assets: &Assets) {
        // Dynamics must always be built into base layer 0.
        self.vm.set_active_vm(0);
        self.frame_counter = self.frame_counter.wrapping_add(1);
        self.avatar_builder
            .set_shading_options(AvatarShadingOptions {
                enabled: self.settings.avatar_shading_enabled,
                skin_enabled: self.settings.avatar_skin_shading_enabled,
            });
        let current_hash = self.dynamics_hash_2d(map, animation_frame);
        let has_active_tile_particles = !self.tile_emitters_2d.is_empty();
        if self.dynamics_ready_2d
            && self.last_dynamics_hash_2d == Some(current_hash)
            && !has_active_tile_particles
        {
            return;
        }
        self.last_dynamics_hash_2d = Some(current_hash);

        self.vm.execute(Atom::ClearDynamics);
        self.vm.execute(Atom::ClearLights);
        let _has_tile_particles = self.rebuild_tile_particles_2d(map, assets);
        let mut active_avatar_geo: FxHashSet<GeoId> = FxHashSet::default();
        let mut active_impact_geo: FxHashSet<GeoId> = FxHashSet::default();

        for item in &map.items {
            let item_pos = Vec2::new(item.position.x, item.position.z);
            let pos = Vec2::new(item_pos.x, item_pos.y);

            if let Some(Value::Light(light)) = item.attributes.get("light") {
                self.vm.execute(Atom::AddLight {
                    id: GeoId::ItemLight(item.id),
                    light: Light::new_pointlight(item.position)
                        .with_color(Vec3::from(light.get_color()))
                        .with_intensity(light.get_intensity())
                        .with_emitting(light.active)
                        .with_start_distance(light.get_start_distance())
                        .with_end_distance(light.get_end_distance())
                        .with_flicker(light.get_flicker()),
                });
            }

            if let Some(Value::Source(source)) = item.attributes.get("source") {
                if item.attributes.get_bool_default("visible", false) {
                    if let Some(tile) = source.tile_from_tile_list(assets) {
                        let geo_id = GeoId::Item(item.id);
                        let anim_start = self.impact_anim_start_for_item(geo_id, item);
                        if anim_start.is_some() {
                            active_impact_geo.insert(geo_id);
                        }
                        let dynamic =
                            DynamicObject::billboard_tile_2d(geo_id, tile.id, pos, 1.0, 1.0)
                                .with_layer(10)
                                .with_anim_start_counter(anim_start);
                        self.vm.execute(Atom::AddDynamic { object: dynamic });
                    }
                }
            }
        }

        for entity in &map.entities {
            let entity_pos = Vec2::new(entity.position.x, entity.position.z);
            let pos = Vec2::new(entity_pos.x, entity_pos.y);

            // Find light on entity
            if let Some(Value::Light(light)) = entity.attributes.get("light") {
                if light.active {
                    self.vm.execute(Atom::AddLight {
                        id: GeoId::ItemLight(entity.id),
                        light: Light::new_pointlight(entity.position)
                            .with_color(Vec3::from(light.get_color()))
                            .with_intensity(light.get_intensity())
                            .with_emitting(light.active)
                            .with_start_distance(light.get_start_distance())
                            .with_end_distance(light.get_end_distance())
                            .with_flicker(light.get_flicker()),
                    });
                }
            }

            // Find light on entity items
            for (_, item) in entity.iter_inventory() {
                if let Some(Value::Light(light)) = item.attributes.get("light") {
                    if light.active {
                        self.vm.execute(Atom::AddLight {
                            id: GeoId::ItemLight(item.id),
                            light: Light::new_pointlight(entity.position)
                                .with_color(Vec3::from(light.get_color()))
                                .with_intensity(light.get_intensity())
                                .with_emitting(light.active)
                                .with_start_distance(light.get_start_distance())
                                .with_end_distance(light.get_end_distance())
                                .with_flicker(light.get_flicker()),
                        });
                    }
                }
            }

            if entity.attributes.get_bool_default("visible", false) {
                let geo_id = GeoId::Character(entity.id);
                if let Some(avatar) = AvatarRuntimeBuilder::find_avatar_for_entity(entity, assets) {
                    let uploaded = self
                        .avatar_builder
                        .ensure_entity_avatar_uploaded_with_direction(
                            &mut self.vm,
                            entity,
                            avatar,
                            assets,
                            animation_frame,
                            geo_id,
                            Some(crate::AvatarDirection::Front),
                        )
                        || self.avatar_builder.ensure_entity_avatar_uploaded(
                            &mut self.vm,
                            entity,
                            avatar,
                            assets,
                            animation_frame,
                            geo_id,
                        );
                    if uploaded {
                        active_avatar_geo.insert(geo_id);
                        let dynamic = DynamicObject::billboard_avatar_2d(geo_id, pos, 1.0, 1.0)
                            .with_layer(20);
                        self.vm.execute(Atom::AddDynamic { object: dynamic });
                        continue;
                    }
                }

                if let Some(Value::Source(source)) = entity.attributes.get("source") {
                    if let Some(tile) = source.tile_from_tile_list(assets) {
                        let dynamic =
                            DynamicObject::billboard_tile_2d(geo_id, tile.id, pos, 1.0, 1.0)
                                .with_layer(20);
                        self.vm.execute(Atom::AddDynamic { object: dynamic });
                    }
                }
            }
        }

        self.avatar_builder
            .remove_stale_avatars(&mut self.vm, &active_avatar_geo);
        self.impact_anim_starts
            .retain(|geo_id, _| active_impact_geo.contains(geo_id));
        self.dynamics_ready_2d = true;
    }

    pub fn build_dynamics_3d(
        &mut self,
        map: &Map,
        camera: &dyn D3Camera,
        animation_frame: usize,
        assets: &Assets,
    ) {
        // Dynamics must always be built into base layer 0.
        self.vm.set_active_vm(0);
        self.avatar_builder
            .set_shading_options(AvatarShadingOptions {
                enabled: self.settings.avatar_shading_enabled,
                skin_enabled: self.settings.avatar_skin_shading_enabled,
            });
        self.frame_counter = self.frame_counter.wrapping_add(1);
        let has_active_campfire_particles = !self.campfire_emitters.is_empty();
        let has_active_tile_particles = !self.tile_emitters_3d.is_empty();
        let has_active_render_billboard_anim = self
            .billboard_anim_states
            .values()
            .any(|state| (state.start_open - state.target_open).abs() > f32::EPSILON);
        let current_hash = self.dynamics_hash_3d(map, camera, animation_frame);
        if self.dynamics_ready_3d
            && self.last_dynamics_hash_3d == Some(current_hash)
            && !has_active_render_billboard_anim
            && !has_active_campfire_particles
            && !has_active_tile_particles
        {
            return;
        }
        self.last_dynamics_hash_3d = Some(current_hash);

        self.vm.execute(Atom::ClearDynamics);
        self.vm.execute(Atom::ClearLights);
        self.add_sector_campfire_lights(map);
        let _has_campfire_particles = self.rebuild_campfire_particles(map, camera, assets);
        let _has_tile_particles = self.rebuild_tile_particles_3d(map, camera, assets);
        let mut active_avatar_geo: FxHashSet<GeoId> = FxHashSet::default();
        let mut active_impact_geo: FxHashSet<GeoId> = FxHashSet::default();

        let basis = camera.basis_vectors();

        // Entities
        for entity in &map.entities {
            if entity.get_mode() == "dead" {
                continue;
            }

            let show_entity = true; // !(entity.is_player() && camera.id() == "firstp");

            if show_entity {
                // Find light on entity
                if let Some(Value::Light(light)) = entity.attributes.get("light") {
                    self.vm.execute(Atom::AddLight {
                        id: GeoId::ItemLight(entity.id),
                        light: Light::new_pointlight(entity.position)
                            .with_color(Vec3::from(light.get_color().map(|c| c.powf(2.2)))) // Convert light to linear
                            .with_intensity(light.get_intensity())
                            .with_emitting(light.active)
                            .with_start_distance(light.get_start_distance())
                            .with_end_distance(light.get_end_distance())
                            .with_flicker(light.get_flicker()),
                    });
                }

                // Find light on entity items
                for (_, item) in entity.iter_inventory() {
                    if let Some(Value::Light(light)) = item.attributes.get("light") {
                        self.vm.execute(Atom::AddLight {
                            id: GeoId::ItemLight(item.id),
                            light: Light::new_pointlight(entity.position)
                                .with_color(Vec3::from(light.get_color().map(|c| c.powf(2.2)))) // Convert light to linear
                                .with_intensity(light.get_intensity())
                                .with_emitting(light.active)
                                .with_start_distance(light.get_start_distance())
                                .with_end_distance(light.get_end_distance())
                                .with_flicker(light.get_flicker()),
                        });
                    }
                }

                let size = entity.attributes.get_float_default("size", 2.0).max(0.01);
                let pos_xz = entity.get_pos_xz();
                let mut ground_y = map
                    .find_sector_at(pos_xz)
                    .map(|s| s.properties.get_float_default("floor_height", 0.0))
                    .unwrap_or(0.0);
                if ground_y == 0.0 {
                    let config = crate::chunkbuilder::terrain_generator::TerrainConfig::default();
                    ground_y =
                        crate::chunkbuilder::terrain_generator::TerrainGenerator::sample_height_at(
                            map, pos_xz, &config,
                        );
                }
                let center3 = Vec3::new(
                    entity.position.x,
                    entity.position.y + size * 0.5,
                    entity.position.z,
                );
                let preview_center3 =
                    Vec3::new(entity.position.x, ground_y + size * 0.5, entity.position.z);
                let geo_id = GeoId::Character(entity.id);
                let mut rendered_avatar = false;
                let visible = entity.attributes.get_bool_default("visible", false);

                if visible
                    && let Some(avatar) =
                        AvatarRuntimeBuilder::find_avatar_for_entity(entity, assets)
                {
                    let direction = if camera.id() == "iso" && entity.is_player() {
                        Self::avatar_direction_from_orientation(entity)
                    } else {
                        Self::avatar_direction_3d(entity, camera)
                    };
                    if self
                        .avatar_builder
                        .ensure_entity_avatar_uploaded_with_direction(
                            &mut self.vm,
                            entity,
                            avatar,
                            assets,
                            animation_frame,
                            geo_id,
                            Some(direction),
                        )
                    {
                        active_avatar_geo.insert(geo_id);
                        let dynamic = DynamicObject::billboard_avatar(
                            geo_id, center3, basis.1, basis.2, size, size,
                        );
                        self.vm.execute(Atom::AddDynamic { object: dynamic });
                        rendered_avatar = true;
                    }
                }

                if !rendered_avatar {
                    if visible
                        && let Some(Value::Source(source)) = entity.attributes.get("source")
                        && let Some(tile) = source.tile_from_tile_list(assets)
                    {
                        let dynamic = DynamicObject::billboard_tile(
                            GeoId::Character(entity.id),
                            tile.id,
                            center3,
                            basis.1,
                            basis.2,
                            size,
                            size,
                        );
                        self.vm.execute(Atom::AddDynamic { object: dynamic });
                    } else {
                        let icon = if Some(entity.creator_id) == map.selected_entity_item {
                            self.character_on
                        } else {
                            self.character_off
                        };
                        let dynamic = DynamicObject::billboard_tile(
                            GeoId::Character(entity.id),
                            icon,
                            preview_center3,
                            basis.1,
                            basis.2,
                            size,
                            size,
                        );
                        self.vm.execute(Atom::AddDynamic { object: dynamic });
                    }
                }
            }

            // Items
            for item in &map.items {
                // Skip items that are bound to a profile/host sector; they are rendered as billboards for gates/doors.
                let is_profile_bound = matches!(
                    item.attributes.get("profile_host_sector_id"),
                    Some(Value::UInt(_))
                ) && matches!(
                    item.attributes.get("profile_sector_id"),
                    Some(Value::UInt(_))
                );
                if is_profile_bound {
                    continue;
                }

                let show_entity = true; // !(entity.is_player() && camera.id() == "firstp");

                if show_entity {
                    if let Some(Value::Light(light)) = item.attributes.get("light") {
                        self.vm.execute(Atom::AddLight {
                            id: GeoId::ItemLight(item.id),
                            light: Light::new_pointlight(item.position)
                                .with_color(Vec3::from(light.get_color().map(|c| c.powf(2.2)))) // Convert light to linear
                                .with_intensity(light.get_intensity())
                                .with_emitting(light.active)
                                .with_start_distance(light.get_start_distance())
                                .with_end_distance(light.get_end_distance())
                                .with_flicker(light.get_flicker()),
                        });
                    }

                    let size = 1.0;
                    let visible = item.attributes.get_bool_default("visible", false);
                    let pos_xz = item.get_pos_xz();
                    let mut ground_y = map
                        .find_sector_at(pos_xz)
                        .map(|s| s.properties.get_float_default("floor_height", 0.0))
                        .unwrap_or(0.0);
                    if ground_y == 0.0 {
                        let config =
                            crate::chunkbuilder::terrain_generator::TerrainConfig::default();
                        ground_y = crate::chunkbuilder::terrain_generator::TerrainGenerator::sample_height_at(
                            map, pos_xz, &config,
                        );
                    }

                    if visible
                        && let Some(Value::Source(source)) = item.attributes.get("source")
                        && let Some(tile) = source.tile_from_tile_list(assets)
                    {
                        let alignment = item
                            .attributes
                            .get_str_default("billboard_alignment", "upright".into())
                            .to_ascii_lowercase();
                        let floor_aligned =
                            matches!(alignment.as_str(), "floor" | "ground" | "flat");
                        let (center3, view_right, view_up) = if floor_aligned {
                            (
                                Vec3::new(item.position.x, ground_y + 0.01, item.position.z),
                                Vec3::unit_x(),
                                Vec3::unit_z(),
                            )
                        } else {
                            let is_spell_like = item.attributes.get_bool_default("is_spell", false)
                                || item.attributes.get_bool_default("spell_impacting", false);
                            let y = if is_spell_like {
                                item.position.y + size * 0.5
                            } else {
                                ground_y + size * 0.5
                            };
                            (
                                Vec3::new(item.position.x, y, item.position.z),
                                basis.1,
                                basis.2,
                            )
                        };

                        let dynamic = DynamicObject::billboard_tile(
                            GeoId::Item(item.id),
                            tile.id,
                            center3,
                            view_right,
                            view_up,
                            size,
                            size,
                        )
                        .with_anim_start_counter({
                            let geo_id = GeoId::Item(item.id);
                            let anim_start = self.impact_anim_start_for_item(geo_id, item);
                            if anim_start.is_some() {
                                active_impact_geo.insert(geo_id);
                            }
                            anim_start
                        });
                        self.vm.execute(Atom::AddDynamic { object: dynamic });
                    } else {
                        let center3 =
                            Vec3::new(item.position.x, ground_y + size * 0.5, item.position.z);
                        let icon = if Some(item.creator_id) == map.selected_entity_item {
                            self.item_on
                        } else {
                            self.item_off
                        };
                        let dynamic = DynamicObject::billboard_tile(
                            GeoId::Item(item.id),
                            icon,
                            center3,
                            basis.1,
                            basis.2,
                            size,
                            size,
                        );
                        self.vm.execute(Atom::AddDynamic { object: dynamic });
                    }
                }
            }
        }

        // Vertices with billboards
        for vertex in &map.vertices {
            if let Some(Value::Source(PixelSource::TileId(tile_id))) =
                vertex.properties.get("source")
            {
                let size = vertex.properties.get_float_default("source_size", 1.0);
                let center3 = Vec3::new(vertex.x, vertex.z + size * 0.5, vertex.y);

                let dynamic = DynamicObject::billboard_tile(
                    GeoId::Vertex(vertex.id),
                    *tile_id,
                    center3,
                    basis.1,
                    basis.2,
                    size,
                    size,
                );
                self.vm.execute(Atom::AddDynamic { object: dynamic });
            }
        }

        // Billboards (doors/gates)
        const BILLBOARD_ANIMATION_DURATION_S: f32 = 0.35;

        // Drop stale animation states for billboards that vanished with chunk updates.
        self.billboard_anim_states
            .retain(|geo_id, _| self.billboards.contains_key(geo_id));

        for (geo_id, billboard) in &self.billboards {
            // Doors/gates use GeoId::Hole(host_sector, profile_sector)
            let resolved_item = match geo_id {
                GeoId::Hole(host, profile) => {
                    Self::find_item_by_profile_attrs(map, Some(*host), Some(*profile))
                }
                _ => None,
            };

            let (is_visible, item_animation, item_duration, item_clock): (
                bool,
                Option<BillboardAnimation>,
                f32,
                AnimationClock,
            ) = resolved_item.map_or(
                (
                    true,
                    None,
                    BILLBOARD_ANIMATION_DURATION_S,
                    AnimationClock::Render,
                ),
                |item| {
                    let visible = item.attributes.get_bool_default("visible", true);
                    // Allow items to override billboard animation: same numeric codes as map surface.
                    let anim_code = item.attributes.get_int_default("billboard_animation", -1);
                    let anim = match anim_code {
                        1 => Some(BillboardAnimation::OpenUp),
                        2 => Some(BillboardAnimation::OpenRight),
                        3 => Some(BillboardAnimation::OpenDown),
                        4 => Some(BillboardAnimation::OpenLeft),
                        5 => Some(BillboardAnimation::Fade),
                        _ => None,
                    };
                    let duration = item
                        .attributes
                        .get_float_default("animation_duration", BILLBOARD_ANIMATION_DURATION_S);
                    let clock = match item
                        .attributes
                        .get_str("animation_clock")
                        .map(|s| s.to_ascii_lowercase())
                    {
                        Some(ref s) if s == "tick" || s == "game" => AnimationClock::GameTick,
                        Some(ref s) if s == "render" || s == "smooth" || s == "frame" => {
                            AnimationClock::Render
                        }
                        _ => AnimationClock::Render,
                    };
                    (visible, anim, duration, clock)
                },
            );

            // Per-item override wins; fall back to the baked-in animation from the profile sector.
            let animation = item_animation.unwrap_or(billboard.animation);
            let duration_s = item_duration;
            let clock = item_clock;

            // Tile override: if the controlling item has a source/tile_id, use it.
            let tile_override = resolved_item.and_then(|item| {
                if let Some(Value::Source(src)) = item.attributes.get("source") {
                    src.tile_from_tile_list(assets).map(|t| t.id)
                } else {
                    None
                }
            });
            let tile_id = tile_override.unwrap_or(billboard.tile_id);

            let (clock_frame, clock_fps) = match clock {
                AnimationClock::Render => (self.frame_counter, self.render_fps),
                AnimationClock::GameTick => (animation_frame, self.game_tick_fps),
            };

            // Opening means the door scrolls away, so open_amount = 1.0 => fully open (invisible).
            let desired_open = if is_visible { 0.0 } else { 1.0 };

            // Track animation state per billboard; initialise from the current desired state.
            let state = self
                .billboard_anim_states
                .entry(*geo_id)
                .or_insert_with(|| BillboardAnimState::new(desired_open, clock_frame));

            // If server toggled visibility, start a new transition from the current pose.
            if (desired_open - state.target_open).abs() > f32::EPSILON {
                let current_open = state.open_amount(clock_frame, clock_fps, duration_s);
                *state = BillboardAnimState {
                    start_open: current_open,
                    target_open: desired_open,
                    start_frame: clock_frame,
                };
            }

            let open_amount = state.open_amount(clock_frame, clock_fps, duration_s);
            if (open_amount - state.target_open).abs() <= 1e-3 {
                state.start_open = state.target_open;
                state.start_frame = clock_frame;
            }

            // Skip fully open for slide/open animations.
            // Fade keeps geometry alive at opacity=0 so it can still occlude sky/sun in Raster3D.
            if open_amount >= 0.999
                && desired_open > 0.5
                && !matches!(animation, BillboardAnimation::Fade)
            {
                continue;
            }

            let mut animated_center = billboard.center;
            let animated_width = billboard.size;
            let animated_height = billboard.size;
            let repeat_mode = billboard.repeat_mode;
            let mut opacity = 1.0_f32;

            match animation {
                BillboardAnimation::OpenUp => {
                    animated_center += billboard.right * (open_amount * billboard.size);
                }
                BillboardAnimation::OpenDown => {
                    animated_center -= billboard.right * (open_amount * billboard.size);
                }
                BillboardAnimation::OpenRight => {
                    animated_center += billboard.up * (open_amount * billboard.size);
                }
                BillboardAnimation::OpenLeft => {
                    animated_center -= billboard.up * (open_amount * billboard.size);
                }
                BillboardAnimation::Fade => {
                    // Pure alpha fade; geometry unchanged.
                    opacity = 1.0 - open_amount;
                }
                BillboardAnimation::None => {
                    if !is_visible {
                        continue;
                    }
                }
            }

            if animated_width <= f32::EPSILON || animated_height <= f32::EPSILON {
                continue;
            }

            let dynamic = DynamicObject::billboard_tile(
                *geo_id,
                tile_id,
                animated_center,
                billboard.up,
                billboard.right,
                animated_width,
                animated_height,
            )
            .with_repeat_mode(repeat_mode)
            .with_opacity(opacity);
            self.vm.execute(Atom::AddDynamic { object: dynamic });
        }

        self.avatar_builder
            .remove_stale_avatars(&mut self.vm, &active_avatar_geo);
        self.impact_anim_starts
            .retain(|geo_id, _| active_impact_geo.contains(geo_id));
        self.dynamics_ready_3d = true;
    }
}
