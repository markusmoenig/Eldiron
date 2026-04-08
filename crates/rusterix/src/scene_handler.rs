use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    str::FromStr,
};

use crate::{
    Assets, AvatarDirection, AvatarShadingOptions, BillboardAnimation, BillboardMetadata, D3Camera,
    Item, Map, ParticleEmitter, PixelSource, RenderSettings, Texture, Tile, Value,
    ValueTomlLoader, avatar_builder::AvatarRuntimeBuilder,
    chunkbuilder::d3chunkbuilder::DEFAULT_TILE_ID,
};
use buildergraph::{BuilderDocument, BuilderOutputTarget, BuilderPrimitive};
use indexmap::IndexMap;
use rust_embed::EmbeddedFile;
use rustc_hash::{FxHashMap, FxHashSet};
use scenevm::{Atom, Chunk, DynamicMeshVertex, DynamicObject, GeoId, Light, SceneVM};
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

#[derive(Clone)]
struct BuilderParticleSource {
    key: u32,
    light_id: GeoId,
    tile_id: Uuid,
    emitter: ParticleEmitter,
    light_override: Option<crate::map::tile::TileLightEmitter>,
    origin: Vec3<f32>,
    direction: Vec3<f32>,
    size_scale: f32,
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

#[derive(Default)]
pub(crate) struct DoorAnimState {
    start_open: f32,
    target_open: f32,
    start_frame: usize,
}

impl DoorAnimState {
    fn new(initial_open: f32, frame: usize) -> Self {
        Self {
            start_open: initial_open,
            target_open: initial_open,
            start_frame: frame,
        }
    }

    fn open_amount(&self, frame: usize, fps: f32, duration_seconds: f32) -> f32 {
        if duration_seconds <= 0.0 {
            return self.target_open;
        }
        let elapsed_seconds = frame.saturating_sub(self.start_frame) as f32 / fps;
        let t = (elapsed_seconds / duration_seconds).clamp(0.0, 1.0);
        let smooth = t * t * (3.0 - 2.0 * t);
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
    pub base_settings: RenderSettings,

    // Billboards for dynamic doors/gates (indexed by GeoId for fast lookup)
    pub billboards: FxHashMap<GeoId, BillboardMetadata>,

    // Animation state per billboard
    pub(crate) billboard_anim_states: FxHashMap<GeoId, BillboardAnimState>,
    pub(crate) door_anim_states: FxHashMap<GeoId, DoorAnimState>,
    // Per-item animation phase starts for one-shot impact effects.
    impact_anim_starts: FxHashMap<GeoId, u32>,
    campfire_emitters: FxHashMap<u32, ParticleEmitter>,
    tile_emitters_2d: FxHashMap<u32, ParticleEmitter>,
    tile_emitters_3d: FxHashMap<u32, ParticleEmitter>,
    builder_emitters_2d: FxHashMap<u32, ParticleEmitter>,
    builder_emitters_3d: FxHashMap<u32, ParticleEmitter>,

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
    pending_particle_steps_2d: usize,
    pending_particle_steps_3d: usize,
    last_dungeon_render_signature: Option<u64>,
}

impl Default for SceneHandler {
    fn default() -> Self {
        SceneHandler::empty()
    }
}

impl SceneHandler {
    const PARTICLE_SIM_FPS: f32 = 15.0;
    const PARTICLE_SIM_STEP: f32 = 1.0 / Self::PARTICLE_SIM_FPS;
    const MAX_PARTICLE_STEPS_PER_BUILD: usize = 8;
    const PARTICLE_TIME_SCALE: f32 = 1.0;

    pub fn tick_particle_clocks(&mut self) {
        self.pending_particle_steps_2d =
            (self.pending_particle_steps_2d + 1).min(Self::MAX_PARTICLE_STEPS_PER_BUILD);
        self.pending_particle_steps_3d =
            (self.pending_particle_steps_3d + 1).min(Self::MAX_PARTICLE_STEPS_PER_BUILD);
    }

    fn advance_emitter(emitter: &mut ParticleEmitter, steps: usize) {
        for _ in 0..steps {
            emitter.update(Self::PARTICLE_SIM_STEP * Self::PARTICLE_TIME_SCALE);
        }
    }

    fn effective_tile_light_range(range: f32) -> f32 {
        (range.max(0.0) * 6.0).max(2.0)
    }

    fn effective_tile_light_start_distance(end_distance: f32) -> f32 {
        if end_distance <= 0.0 {
            0.0
        } else {
            (end_distance * 0.35).clamp(0.0, end_distance * 0.9)
        }
    }

    fn effective_tile_light_intensity(intensity: f32) -> f32 {
        intensity.max(0.0) * 4.0
    }

    fn particle_sprite_tile_id(tile_id: Uuid) -> Uuid {
        Uuid::from_u128(tile_id.as_u128() ^ 0x705f_6172_7469_636c_655f_7370_7269)
    }

    fn build_particle_sprite_texture(color: [u8; 4], ramp: Option<&[[u8; 4]; 4]>) -> Texture {
        let size = 16usize;
        let mut data = vec![0u8; size * size * 4];
        let center = (size as f32 - 1.0) * 0.5;
        let radius = center.max(1.0);
        let ramp = ramp
            .copied()
            .unwrap_or_else(|| Self::derive_particle_ramp(color));

        for y in 0..size {
            for x in 0..size {
                let dx = (x as f32 - center) / radius;
                let dy = (y as f32 - center) / radius;
                let dist = (dx * dx + dy * dy).sqrt();
                let radial = (1.0 - dist).clamp(0.0, 1.0);
                let alpha = (radial.powf(1.7) * 255.0) as u8;
                let height_t =
                    (1.0 - (y as f32 / (size.saturating_sub(1).max(1) as f32))).clamp(0.0, 1.0);
                let ramp_t =
                    ((1.0 - height_t) * 0.75 + dist.clamp(0.0, 1.0) * 0.25).clamp(0.0, 0.999);
                let scaled = ramp_t * 3.0;
                let idx0 = scaled.floor() as usize;
                let idx1 = (idx0 + 1).min(3);
                let frac = scaled.fract();
                let idx = (y * size + x) * 4;
                let shade = (0.82 + radial * 0.18).clamp(0.0, 1.0);
                data[idx] = ((ramp[idx0][0] as f32 * (1.0 - frac) + ramp[idx1][0] as f32 * frac)
                    * shade)
                    .clamp(0.0, 255.0) as u8;
                data[idx + 1] =
                    ((ramp[idx0][1] as f32 * (1.0 - frac) + ramp[idx1][1] as f32 * frac) * shade)
                        .clamp(0.0, 255.0) as u8;
                data[idx + 2] =
                    ((ramp[idx0][2] as f32 * (1.0 - frac) + ramp[idx1][2] as f32 * frac) * shade)
                        .clamp(0.0, 255.0) as u8;
                data[idx + 3] = alpha;
            }
        }

        let mut texture = Texture::new(data, size, size);
        // Particle billboards should read as self-lit sprites instead of generic shaded tiles.
        texture.set_materials_all(0.0, 0.0, 1.0, 1.0);
        texture
    }

    fn derive_particle_ramp(base: [u8; 4]) -> [[u8; 4]; 4] {
        [
            [
                (base[0] as f32 * 1.15).clamp(0.0, 255.0) as u8,
                (base[1] as f32 * 1.1).clamp(0.0, 255.0) as u8,
                (base[2] as f32 * 0.9 + 24.0).clamp(0.0, 255.0) as u8,
                255,
            ],
            base,
            [
                (base[0] as f32 * 0.75).clamp(0.0, 255.0) as u8,
                (base[1] as f32 * 0.45).clamp(0.0, 255.0) as u8,
                (base[2] as f32 * 0.3).clamp(0.0, 255.0) as u8,
                255,
            ],
            [36, 32, 32, 255],
        ]
    }

    pub fn sync_base_render_settings(&mut self, config: &str) {
        let mut base = RenderSettings::default();
        _ = base.read(config);
        self.base_settings = base.clone();
        self.settings = base;
        self.last_dungeon_render_signature = None;
    }

    fn sector_floor_height_for_player(map: &Map, sector: &crate::Sector) -> Option<f32> {
        if map
            .get_surface_for_sector_id(sector.id)
            .map(|surface| surface.plane.normal.y.abs() <= 0.7)
            .unwrap_or(true)
        {
            return None;
        }
        if sector.properties.get_float_default("roof_height", 0.0) > 0.0 {
            return None;
        }

        let mut vertex_ids: FxHashSet<u32> = FxHashSet::default();
        let mut sum_y = 0.0f32;
        let mut count = 0usize;
        for linedef_id in &sector.linedefs {
            let Some(ld) = map.find_linedef(*linedef_id) else {
                continue;
            };
            for vertex_id in [ld.start_vertex, ld.end_vertex] {
                if vertex_ids.insert(vertex_id)
                    && let Some(v) = map.get_vertex_3d(vertex_id)
                {
                    sum_y += v.y;
                    count += 1;
                }
            }
        }
        if count == 0 {
            None
        } else {
            Some(sum_y / count as f32)
        }
    }

    fn current_player_sector<'a>(map: &'a Map) -> Option<&'a crate::Sector> {
        let player = map.entities.iter().find(|entity| entity.is_player())?;
        let player_pos = player.get_pos_xz();
        let reference_y = player.position.y;
        let mut best_below: Option<(&crate::Sector, f32)> = None;
        let mut best_above: Option<(&crate::Sector, f32)> = None;
        const FLOOR_EPS: f32 = 0.05;

        for sector in map
            .sectors
            .iter()
            .filter(|s| s.layer.is_none() && s.is_inside(map, player_pos))
        {
            let Some(h) = Self::sector_floor_height_for_player(map, sector) else {
                continue;
            };
            if h <= reference_y + FLOOR_EPS {
                if best_below.is_none_or(|(_, curr_h)| h > curr_h) {
                    best_below = Some((sector, h));
                }
            } else {
                let dist = h - reference_y;
                if best_above.is_none_or(|(_, curr_dist)| dist < curr_dist) {
                    best_above = Some((sector, dist));
                }
            }
        }

        best_below
            .map(|(sector, _)| sector)
            .or_else(|| best_above.map(|(sector, _)| sector))
            .or_else(|| map.find_sector_at(player_pos))
    }

    fn dungeon_render_signature(map: &Map, active: bool) -> u64 {
        let mut hasher = DefaultHasher::new();
        active.hash(&mut hasher);
        Self::dungeon_render_toml(map).hash(&mut hasher);
        if let Some(sector) = Self::current_player_sector(map) {
            sector.id.hash(&mut hasher);
        }
        hasher.finish()
    }

    fn dungeon_render_toml(map: &Map) -> String {
        if let Some(render_toml) = map.properties.get_str("dungeon_render_toml")
            && !render_toml.trim().is_empty()
        {
            return render_toml.to_string();
        }

        if map.properties.get("dungeon_render_transition_seconds").is_some()
            || map.properties.get("dungeon_render_sun_enabled").is_some()
            || map.properties.get("dungeon_render_shadow_enabled").is_some()
            || map.properties.get("dungeon_render_fog_density").is_some()
            || map.properties.get("dungeon_render_fog_color").is_some()
        {
            return format!(
                "[render]\ntransition_seconds = {}\nsun_enabled = {}\nshadow_enabled = {}\nfog_density = {}\nfog_color = \"{}\"\n",
                map.properties
                    .get_float_default("dungeon_render_transition_seconds", 1.0),
                map.properties
                    .get_bool_default("dungeon_render_sun_enabled", false),
                map.properties
                    .get_bool_default("dungeon_render_shadow_enabled", true),
                map.properties
                    .get_float_default("dungeon_render_fog_density", 5.0),
                map.properties
                    .get_str_default("dungeon_render_fog_color", "#000000".to_string()),
            );
        }

        "[render]\ntransition_seconds = 1.0\nsun_enabled = false\nshadow_enabled = true\nfog_density = 5.0\nfog_color = \"#000000\"\n".to_string()
    }

    pub fn apply_dungeon_render_overrides(&mut self, map: &Map) {
        let current_sector = Self::current_player_sector(map);
        let in_dungeon = current_sector
            .map(|sector| {
                sector
                    .properties
                    .get_str_default("generated_by", String::new())
                    == "dungeon_tool"
            })
            .unwrap_or(false);

        let signature = Self::dungeon_render_signature(map, in_dungeon);
        if self.last_dungeon_render_signature == Some(signature) {
            return;
        }
        self.last_dungeon_render_signature = Some(signature);

        let render_toml = Self::dungeon_render_toml(map);
        let render_group = if in_dungeon {
            ValueTomlLoader::from_str(&render_toml)
                .ok()
                .and_then(|groups| groups.get("render").cloned())
        } else {
            None
        };
        let transition = render_group
            .as_ref()
            .and_then(|render| render.get_float("transition_seconds"))
            .unwrap_or(1.0)
            .max(0.0);

        for name in RenderSettings::runtime_override_names() {
            let mut target = render_group
                .as_ref()
                .and_then(|render| render.get(name).cloned())
                .or_else(|| self.base_settings.value_for_name(name));
            if *name == "fog_density"
                && let Some(Value::Float(v)) = target.as_mut()
                && render_group
                    .as_ref()
                    .and_then(|render| render.get(name))
                    .is_some()
            {
                *v /= 100.0;
            }
            if let Some(target) = target {
                let _ = self.settings.set(name, target, transition);
            }
        }
    }

    fn rebuild_campfire_particles(
        &mut self,
        map: &Map,
        camera: &dyn D3Camera,
        assets: &Assets,
        particle_steps: usize,
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
            Self::advance_emitter(emitter, particle_steps);

            for (index, particle) in emitter.particles.iter().enumerate() {
                has_particles = true;
                let opacity = (particle.lifetime / emitter.lifetime_range.1).clamp(0.0, 1.0);
                let size = (particle.radius * 2.8).max(0.08);
                let center = particle.pos + Vec3::new(0.0, size * 0.35, 0.0);
                let tint = Vec3::new(
                    (particle.color[0] as f32 / 255.0).powf(2.2),
                    (particle.color[1] as f32 / 255.0).powf(2.2),
                    (particle.color[2] as f32 / 255.0).powf(2.2),
                );
                let dynamic = DynamicObject::particle_tile(
                    GeoId::Unknown(sector.id.saturating_mul(1024).saturating_add(index as u32)),
                    flame_tile_id,
                    center,
                    basis.1,
                    basis.2,
                    size,
                    size * 2.1,
                )
                .with_tint(tint)
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

    fn hash_u32_label(label: &str) -> u32 {
        label
            .bytes()
            .fold(0u32, |acc, byte| acc.wrapping_mul(16777619) ^ byte as u32)
    }

    fn normalize_builder_material_key(name: &str) -> String {
        let mut out = String::new();
        let mut prev_is_sep = false;
        for (i, ch) in name.chars().enumerate() {
            if ch.is_ascii_alphanumeric() {
                if ch.is_ascii_uppercase() {
                    if i > 0 && !prev_is_sep {
                        out.push('_');
                    }
                    out.push(ch.to_ascii_lowercase());
                } else {
                    out.push(ch.to_ascii_lowercase());
                }
                prev_is_sep = false;
            } else if !prev_is_sep && !out.is_empty() {
                out.push('_');
                prev_is_sep = true;
            }
        }
        out.trim_matches('_').to_string()
    }

    fn builder_material_tile(
        properties: &crate::ValueContainer,
        material_slot: &str,
        assets: &Assets,
    ) -> Option<Tile> {
        let key = format!(
            "builder_material_{}",
            Self::normalize_builder_material_key(material_slot)
        );
        if let Some(Value::Source(ps)) = properties.get(&key) {
            return ps.tile_from_tile_list(assets);
        }
        match properties.get("source") {
            Some(Value::Source(ps)) => ps.tile_from_tile_list(assets),
            _ => None,
        }
    }

    fn builder_linedef_particle_sources(map: &Map, assets: &Assets) -> Vec<BuilderParticleSource> {
        fn rotate_vec3_x(v: Vec3<f32>, angle: f32) -> Vec3<f32> {
            let (s, c) = angle.sin_cos();
            Vec3::new(v.x, v.y * c - v.z * s, v.y * s + v.z * c)
        }

        fn rotate_vec3_y(v: Vec3<f32>, angle: f32) -> Vec3<f32> {
            let (s, c) = angle.sin_cos();
            Vec3::new(v.x * c - v.z * s, v.y, v.x * s + v.z * c)
        }

        fn builder_linedef_outward(
            map: &Map,
            linedef: &crate::Linedef,
            along: Vec3<f32>,
        ) -> Vec3<f32> {
            let explicit = Vec3::new(
                linedef
                    .properties
                    .get_float_default("builder_graph_outward_x", 0.0),
                linedef
                    .properties
                    .get_float_default("builder_graph_outward_y", 0.0),
                linedef
                    .properties
                    .get_float_default("builder_graph_outward_z", 0.0),
            );
            if let Some(outward) = explicit.try_normalized() {
                return outward;
            }
            let mut outward = Vec3::new(-along.z, 0.0, along.x);
            let side = linedef
                .properties
                .get_float_default("builder_graph_wall_side", 0.0);
            if side.abs() <= 1e-5 {
                let preferred_sector = linedef.sector_ids.first().copied();
                if let Some(sector_id) = preferred_sector
                    && let Some(sector) = map.find_sector(sector_id)
                    && let Some(center) = sector.center(map)
                    && let Some(dist) = linedef.signed_distance(map, center)
                    && dist.abs() > 1e-5
                {
                    outward *= if dist >= 0.0 { -1.0 } else { 1.0 };
                    return outward;
                }
            }
            if side < 0.0 {
                outward = -outward;
            }
            outward
        }

        fn builder_linedef_along(linedef: &crate::Linedef, fallback_along: Vec3<f32>) -> Vec3<f32> {
            let explicit = Vec3::new(
                linedef.properties.get_float_default("host_along_x", 0.0),
                linedef.properties.get_float_default("host_along_y", 0.0),
                linedef.properties.get_float_default("host_along_z", 0.0),
            );
            if let Some(along) = explicit.try_normalized() {
                return along;
            }
            fallback_along
        }

        let mut out = Vec::new();

        for linedef in &map.linedefs {
            let builder_graph_data = linedef
                .properties
                .get_str_default("builder_graph_data", String::new());
            if builder_graph_data.trim().is_empty() {
                continue;
            }
            let Ok(graph) = BuilderDocument::from_text(&builder_graph_data) else {
                continue;
            };
            let spec = graph.output_spec();
            if spec.target != BuilderOutputTarget::Linedef || spec.host_refs != 1 {
                continue;
            }

            let Some(v0) = map.get_vertex_3d(linedef.start_vertex) else {
                continue;
            };
            let Some(v1) = map.get_vertex_3d(linedef.end_vertex) else {
                continue;
            };

            let origin = (v0 + v1) * 0.5;
            let mut along = Vec3::new(v1.x - v0.x, 0.0, v1.z - v0.z);
            let span_length = along.magnitude().max(1e-5);
            if along.magnitude() <= 1e-5 {
                along = Vec3::new(1.0, 0.0, 0.0);
            } else {
                along = along.normalized();
            }
            along = builder_linedef_along(linedef, along);
            let outward = builder_linedef_outward(map, linedef, along);
            let up = Vec3::new(0.0, 1.0, 0.0);
            let wall_height = linedef
                .properties
                .get_float_default("wall_height", 2.0)
                .max(0.01);
            let wall_epsilon = linedef
                .properties
                .get_float_default("profile_wall_epsilon", 0.001)
                .max(0.0);
            let surface_origin = match (
                linedef
                    .properties
                    .get_float("builder_graph_surface_origin_x"),
                linedef
                    .properties
                    .get_float("builder_graph_surface_origin_y"),
                linedef
                    .properties
                    .get_float("builder_graph_surface_origin_z"),
            ) {
                (Some(x), Some(y), Some(z)) => Some(Vec3::new(x, y, z)),
                _ => None,
            };
            let face_offset = linedef.properties.get_float("builder_graph_face_offset");
            let face_origin = if let Some(face_offset) = face_offset {
                origin + outward * face_offset.max(wall_epsilon)
            } else if let Some(surface_origin) = surface_origin {
                surface_origin
            } else {
                let wall_offset = linedef
                    .properties
                    .get_float_default("wall_width", 0.0)
                    .max(0.0)
                    * 0.5
                    + wall_epsilon;
                origin + outward * wall_offset
            };
            let host_origin = face_origin - up * (wall_height * 0.5) + outward * wall_epsilon;

            let Ok(assembly) = graph.evaluate() else {
                continue;
            };

            for (primitive_index, primitive) in assembly.primitives.iter().enumerate() {
                let (material_slot, emitter_origin, direction) = match primitive {
                    BuilderPrimitive::Box {
                        size,
                        transform,
                        material_slot,
                        host_position_normalized,
                        host_position_y_normalized,
                        host_scale_y_normalized,
                        host_scale_x_normalized,
                        ..
                    } => {
                        let sx = if *host_scale_x_normalized {
                            transform.scale.x * span_length
                        } else {
                            transform.scale.x
                        };
                        let scaled = Vec3::new(
                            size.x * sx,
                            size.y
                                * if *host_scale_y_normalized {
                                    transform.scale.y * wall_height
                                } else {
                                    transform.scale.y
                                },
                            size.z * transform.scale.z,
                        );
                        let tx = if *host_position_normalized {
                            transform.translation.x * span_length
                        } else {
                            transform.translation.x
                        };
                        let ty = if *host_position_y_normalized {
                            transform.translation.y * wall_height
                        } else {
                            transform.translation.y
                        };
                        let center = host_origin
                            + along * tx
                            + up * (ty + scaled.y * 0.5)
                            + outward * transform.translation.z;
                        let tip_local = rotate_vec3_y(
                            rotate_vec3_x(
                                Vec3::new(0.0, scaled.y * 0.5 + 0.02, 0.02),
                                transform.rotation_x,
                            ),
                            transform.rotation_y,
                        );
                        let emitter_origin =
                            center + along * tip_local.x + up * tip_local.y + outward * tip_local.z;
                        let dir_local = rotate_vec3_y(
                            rotate_vec3_x(Vec3::new(0.0, 1.0, 0.0), transform.rotation_x),
                            transform.rotation_y,
                        );
                        let direction =
                            (along * dir_local.x + up * dir_local.y + outward * dir_local.z)
                                .try_normalized()
                                .unwrap_or(up);
                        (material_slot.as_deref(), emitter_origin, direction)
                    }
                    BuilderPrimitive::Cylinder {
                        length,
                        radius: _,
                        transform,
                        material_slot,
                        host_position_normalized,
                        host_position_y_normalized,
                        host_scale_y_normalized,
                        host_scale_x_normalized: _,
                        host_scale_z_normalized: _,
                    } => {
                        let scaled_length = if *host_scale_y_normalized {
                            *length * transform.scale.y * wall_height
                        } else {
                            *length * transform.scale.y
                        };
                        let tx = if *host_position_normalized {
                            transform.translation.x * span_length
                        } else {
                            transform.translation.x
                        };
                        let ty = if *host_position_y_normalized {
                            transform.translation.y * wall_height
                        } else {
                            transform.translation.y
                        };
                        let center = host_origin
                            + along * tx
                            + up * (ty + scaled_length * 0.5)
                            + outward * transform.translation.z;
                        let tip_local = rotate_vec3_y(
                            rotate_vec3_x(
                                Vec3::new(0.0, scaled_length * 0.5 + 0.02, 0.0),
                                transform.rotation_x,
                            ),
                            transform.rotation_y,
                        );
                        let emitter_origin =
                            center + along * tip_local.x + up * tip_local.y + outward * tip_local.z;
                        let dir_local = rotate_vec3_y(
                            rotate_vec3_x(Vec3::new(0.0, 1.0, 0.0), transform.rotation_x),
                            transform.rotation_y,
                        );
                        let direction =
                            (along * dir_local.x + up * dir_local.y + outward * dir_local.z)
                                .try_normalized()
                                .unwrap_or(up);
                        (material_slot.as_deref(), emitter_origin, direction)
                    }
                };
                let Some(material_slot) = material_slot else {
                    continue;
                };
                let Some(tile) =
                    Self::builder_material_tile(&linedef.properties, material_slot, assets)
                else {
                    continue;
                };
                let Some(emitter) = tile.particle_emitter.clone() else {
                    continue;
                };

                let slot_hash =
                    Self::hash_u32_label(material_slot).wrapping_add(primitive_index as u32);
                let key = Self::tile_particle_key(6, linedef.id, slot_hash);

                out.push(BuilderParticleSource {
                    key,
                    light_id: GeoId::Unknown(0xB17D_0000 ^ key),
                    tile_id: tile.id,
                    emitter,
                    light_override: tile.light_emitter.clone(),
                    origin: emitter_origin,
                    direction,
                    size_scale: 1.0,
                });
            }
        }

        out
    }

    fn builder_vertex_particle_sources(map: &Map, assets: &Assets) -> Vec<BuilderParticleSource> {
        fn rotate_vec3_x(v: Vec3<f32>, angle: f32) -> Vec3<f32> {
            let (s, c) = angle.sin_cos();
            Vec3::new(v.x, v.y * c - v.z * s, v.y * s + v.z * c)
        }

        fn rotate_vec3_y(v: Vec3<f32>, angle: f32) -> Vec3<f32> {
            let (s, c) = angle.sin_cos();
            Vec3::new(v.x * c - v.z * s, v.y, v.x * s + v.z * c)
        }

        let mut out = Vec::new();

        for vertex in &map.vertices {
            let builder_graph_data = vertex
                .properties
                .get_str_default("builder_graph_data", String::new());
            if builder_graph_data.trim().is_empty() {
                continue;
            }
            let Ok(graph) = BuilderDocument::from_text(&builder_graph_data) else {
                continue;
            };
            let spec = graph.output_spec();
            if spec.target != BuilderOutputTarget::VertexPair || spec.host_refs != 1 {
                continue;
            }

            let origin = match (
                vertex.properties.get_float("host_surface_origin_x"),
                vertex.properties.get_float("host_surface_origin_y"),
                vertex.properties.get_float("host_surface_origin_z"),
            ) {
                (Some(x), Some(y), Some(z)) => Vec3::new(x, y, z),
                _ => vertex.as_vec3_world(),
            };
            let along = Vec3::new(
                vertex.properties.get_float_default("host_along_x", 1.0),
                vertex.properties.get_float_default("host_along_y", 0.0),
                vertex.properties.get_float_default("host_along_z", 0.0),
            )
            .try_normalized()
            .unwrap_or(Vec3::new(1.0, 0.0, 0.0));
            let outward = Vec3::new(
                vertex.properties.get_float_default("host_outward_x", 0.0),
                vertex.properties.get_float_default("host_outward_y", 0.0),
                vertex.properties.get_float_default("host_outward_z", 1.0),
            )
            .try_normalized()
            .unwrap_or(Vec3::new(0.0, 0.0, 1.0));
            let up = Vec3::new(0.0, 1.0, 0.0);

            let Ok(assembly) = graph.evaluate() else {
                continue;
            };

            for (primitive_index, primitive) in assembly.primitives.iter().enumerate() {
                let (material_slot, emitter_origin, direction) = match primitive {
                    BuilderPrimitive::Box {
                        size,
                        transform,
                        material_slot,
                        ..
                    } => {
                        let scaled = Vec3::new(
                            size.x * transform.scale.x,
                            size.y * transform.scale.y,
                            size.z * transform.scale.z,
                        );
                        let center = origin
                            + along * transform.translation.x
                            + up * (transform.translation.y + scaled.y * 0.5)
                            + outward * transform.translation.z;
                        let tip_local = rotate_vec3_y(
                            rotate_vec3_x(
                                Vec3::new(0.0, scaled.y * 0.5 + 0.02, 0.02),
                                transform.rotation_x,
                            ),
                            transform.rotation_y,
                        );
                        let emitter_origin =
                            center + along * tip_local.x + up * tip_local.y + outward * tip_local.z;
                        let dir_local = rotate_vec3_y(
                            rotate_vec3_x(Vec3::new(0.0, 1.0, 0.0), transform.rotation_x),
                            transform.rotation_y,
                        );
                        let direction =
                            (along * dir_local.x + up * dir_local.y + outward * dir_local.z)
                                .try_normalized()
                                .unwrap_or(up);
                        (material_slot.as_deref(), emitter_origin, direction)
                    }
                    BuilderPrimitive::Cylinder {
                        length,
                        transform,
                        material_slot,
                        ..
                    } => {
                        let scaled_length = *length * transform.scale.y;
                        let center = origin
                            + along * transform.translation.x
                            + up * (transform.translation.y + scaled_length * 0.5)
                            + outward * transform.translation.z;
                        let tip_local = rotate_vec3_y(
                            rotate_vec3_x(
                                Vec3::new(0.0, scaled_length * 0.5 + 0.02, 0.0),
                                transform.rotation_x,
                            ),
                            transform.rotation_y,
                        );
                        let emitter_origin =
                            center + along * tip_local.x + up * tip_local.y + outward * tip_local.z;
                        let dir_local = rotate_vec3_y(
                            rotate_vec3_x(Vec3::new(0.0, 1.0, 0.0), transform.rotation_x),
                            transform.rotation_y,
                        );
                        let direction =
                            (along * dir_local.x + up * dir_local.y + outward * dir_local.z)
                                .try_normalized()
                                .unwrap_or(up);
                        (material_slot.as_deref(), emitter_origin, direction)
                    }
                };
                let Some(material_slot) = material_slot else {
                    continue;
                };
                let Some(tile) =
                    Self::builder_material_tile(&vertex.properties, material_slot, assets)
                else {
                    continue;
                };
                let Some(emitter) = tile.particle_emitter.clone() else {
                    continue;
                };

                let slot_hash =
                    Self::hash_u32_label(material_slot).wrapping_add(primitive_index as u32);
                let key = Self::tile_particle_key(7, vertex.id, slot_hash);

                out.push(BuilderParticleSource {
                    key,
                    light_id: GeoId::Unknown(0xB17E_0000 ^ key),
                    tile_id: tile.id,
                    emitter,
                    light_override: tile.light_emitter.clone(),
                    origin: emitter_origin,
                    direction,
                    size_scale: 1.0,
                });
            }
        }

        out
    }

    fn rebuild_builder_particles_2d(
        &mut self,
        map: &Map,
        assets: &Assets,
        particle_steps: usize,
    ) -> bool {
        let mut active_emitters: FxHashSet<u32> = FxHashSet::default();
        let mut has_particles = false;

        for source in Self::builder_linedef_particle_sources(map, assets)
            .into_iter()
            .chain(Self::builder_vertex_particle_sources(map, assets).into_iter())
        {
            active_emitters.insert(source.key);
            let emitter = self
                .builder_emitters_2d
                .entry(source.key)
                .or_insert_with(|| {
                    let mut emitter = source.emitter.clone();
                    emitter.origin = source.origin;
                    emitter.direction = source.direction;
                    emitter.time_accum = 0.0;
                    emitter.particles.clear();
                    emitter
                });
            emitter.origin = source.origin;
            emitter.direction = source.direction;
            Self::advance_emitter(emitter, particle_steps);

            let lifetime_max = emitter.lifetime_range.1.max(0.001);
            for (index, particle) in emitter.particles.iter().enumerate() {
                has_particles = true;
                let opacity = (particle.lifetime / lifetime_max).clamp(0.0, 1.0);
                let size = (particle.radius * 2.6 * source.size_scale).max(0.12);
                let tint = Vec3::new(
                    (particle.color[0] as f32 / 255.0).powf(2.2),
                    (particle.color[1] as f32 / 255.0).powf(2.2),
                    (particle.color[2] as f32 / 255.0).powf(2.2),
                );
                let dynamic = DynamicObject::particle_tile_2d(
                    GeoId::Unknown(source.key.wrapping_mul(2048).wrapping_add(index as u32)),
                    Self::particle_sprite_tile_id(source.tile_id),
                    Vec2::new(particle.pos.x, particle.pos.z),
                    size,
                    size,
                )
                .with_tint(tint)
                .with_layer(24)
                .with_opacity(opacity);
                self.vm.execute(Atom::AddDynamic { object: dynamic });
            }
        }

        self.builder_emitters_2d
            .retain(|key, _| active_emitters.contains(key));
        has_particles
    }

    fn rebuild_builder_particles_3d(
        &mut self,
        map: &Map,
        camera: &dyn D3Camera,
        assets: &Assets,
        particle_steps: usize,
    ) -> bool {
        let basis = camera.basis_vectors();
        let mut active_emitters: FxHashSet<u32> = FxHashSet::default();
        let mut has_particles = false;

        for source in Self::builder_linedef_particle_sources(map, assets)
            .into_iter()
            .chain(Self::builder_vertex_particle_sources(map, assets).into_iter())
        {
            active_emitters.insert(source.key);
            let emitter = self
                .builder_emitters_3d
                .entry(source.key)
                .or_insert_with(|| {
                    let mut emitter = source.emitter.clone();
                    emitter.origin = source.origin;
                    emitter.direction = source.direction;
                    emitter.time_accum = 0.0;
                    emitter.particles.clear();
                    emitter
                });
            emitter.origin = source.origin;
            emitter.direction = source.direction;
            Self::advance_emitter(emitter, particle_steps);

            let (light_color, light_intensity, light_range, light_flicker, light_lift) =
                if let Some(light) = &source.light_override {
                    let light_range = Self::effective_tile_light_range(light.range);
                    (
                        Vec3::new(
                            (light.color[0] as f32 / 255.0).powf(2.2),
                            (light.color[1] as f32 / 255.0).powf(2.2),
                            (light.color[2] as f32 / 255.0).powf(2.2),
                        ),
                        Self::effective_tile_light_intensity(light.intensity),
                        light_range,
                        light.flicker,
                        light.lift,
                    )
                } else {
                    let light_range = Self::effective_tile_light_range(4.0);
                    (
                        Vec3::new(
                            (source.emitter.color[0] as f32 / 255.0).powf(2.2),
                            (source.emitter.color[1] as f32 / 255.0).powf(2.2),
                            (source.emitter.color[2] as f32 / 255.0).powf(2.2),
                        ),
                        Self::effective_tile_light_intensity(1.8),
                        light_range,
                        0.2,
                        0.06,
                    )
                };
            self.vm.execute(Atom::AddLight {
                id: source.light_id,
                light: Light::new_pointlight(source.origin + Vec3::new(0.0, light_lift, 0.0))
                    .with_color(light_color)
                    .with_intensity(light_intensity)
                    .with_emitting(true)
                    .with_start_distance(Self::effective_tile_light_start_distance(light_range))
                    .with_end_distance(light_range)
                    .with_flicker(light_flicker),
            });

            let lifetime_max = emitter.lifetime_range.1.max(0.001);
            for (index, particle) in emitter.particles.iter().enumerate() {
                has_particles = true;
                let opacity = (particle.lifetime / lifetime_max).clamp(0.0, 1.0);
                let size = (particle.radius * 4.6 * source.size_scale).max(0.28);
                let center = particle.pos + Vec3::new(0.0, size * 0.2, 0.0);
                let tint = Vec3::new(
                    (particle.color[0] as f32 / 255.0).powf(2.2),
                    (particle.color[1] as f32 / 255.0).powf(2.2),
                    (particle.color[2] as f32 / 255.0).powf(2.2),
                );
                let dynamic = DynamicObject::particle_tile(
                    GeoId::Unknown(source.key.wrapping_mul(2048).wrapping_add(index as u32)),
                    Self::particle_sprite_tile_id(source.tile_id),
                    center,
                    basis.1,
                    basis.2,
                    size,
                    size,
                )
                .with_tint(tint)
                .with_opacity(opacity);
                self.vm.execute(Atom::AddDynamic { object: dynamic });
            }
            if source.emitter.flame_base {
                let ramp = source
                    .emitter
                    .color_ramp
                    .unwrap_or([source.emitter.color; 4]);
                let base_size = ((source.emitter.radius_range.0 + source.emitter.radius_range.1)
                    * 0.5
                    * 7.5
                    * source.size_scale)
                    .max(0.42);
                for (layer, (color, scale, yoff, opacity)) in [
                    (ramp[1], 1.0f32, base_size * 0.03, 0.98f32),
                    (ramp[0], 0.82f32, base_size * 0.14, 1.0f32),
                ]
                .into_iter()
                .enumerate()
                {
                    let tint = Vec3::new(
                        (color[0] as f32 / 255.0).powf(2.2),
                        (color[1] as f32 / 255.0).powf(2.2),
                        (color[2] as f32 / 255.0).powf(2.2),
                    );
                    let dynamic = DynamicObject::particle_tile(
                        GeoId::Unknown(
                            source
                                .key
                                .wrapping_mul(4096)
                                .wrapping_add(3000 + layer as u32),
                        ),
                        Self::particle_sprite_tile_id(source.tile_id),
                        source.origin + Vec3::new(0.0, yoff, 0.0),
                        basis.1,
                        basis.2,
                        base_size * scale,
                        base_size * (1.9 - layer as f32 * 0.15) * scale,
                    )
                    .with_tint(tint)
                    .with_opacity(opacity);
                    self.vm.execute(Atom::AddDynamic { object: dynamic });
                }
            }
        }

        self.builder_emitters_3d
            .retain(|key, _| active_emitters.contains(key));
        has_particles
    }

    fn rebuild_tile_particles_2d(
        &mut self,
        map: &Map,
        assets: &Assets,
        particle_steps: usize,
    ) -> bool {
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
            Self::advance_emitter(emitter, particle_steps);

            let lifetime_max = emitter.lifetime_range.1.max(0.001);
            for (index, particle) in emitter.particles.iter().enumerate() {
                *has_particles = true;
                let opacity = (particle.lifetime / lifetime_max).clamp(0.0, 1.0);
                let size = (particle.radius * 1.8 * size_scale).max(0.08);
                let tint = Vec3::new(
                    (particle.color[0] as f32 / 255.0).powf(2.2),
                    (particle.color[1] as f32 / 255.0).powf(2.2),
                    (particle.color[2] as f32 / 255.0).powf(2.2),
                );
                let dynamic = DynamicObject::particle_tile_2d(
                    GeoId::Unknown(key.wrapping_mul(1024).wrapping_add(index as u32)),
                    Self::particle_sprite_tile_id(tile.id),
                    Vec2::new(particle.pos.x, particle.pos.y),
                    size,
                    size,
                )
                .with_tint(tint)
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
        particle_steps: usize,
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
            if let Some(light) = &tile.light_emitter {
                let linear = Vec3::new(
                    (light.color[0] as f32 / 255.0).powf(2.2),
                    (light.color[1] as f32 / 255.0).powf(2.2),
                    (light.color[2] as f32 / 255.0).powf(2.2),
                );
                let end_distance = Self::effective_tile_light_range(light.range);
                vm.execute(Atom::AddLight {
                    id: GeoId::Unknown(0x71A0_0000 ^ key),
                    light: Light::new_pointlight(origin + Vec3::new(0.0, light.lift, 0.0))
                        .with_color(linear)
                        .with_intensity(Self::effective_tile_light_intensity(light.intensity))
                        .with_emitting(true)
                        .with_start_distance(Self::effective_tile_light_start_distance(
                            end_distance,
                        ))
                        .with_end_distance(end_distance)
                        .with_flicker(light.flicker),
                });
            }
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
            Self::advance_emitter(emitter, particle_steps);

            let lifetime_max = emitter.lifetime_range.1.max(0.001);
            for (index, particle) in emitter.particles.iter().enumerate() {
                *has_particles = true;
                let opacity = (particle.lifetime / lifetime_max).clamp(0.0, 1.0);
                let size = (particle.radius * 1.9 * size_scale).max(0.08);
                let center = particle.pos + Vec3::new(0.0, size * 0.2, 0.0);
                let tint = Vec3::new(
                    (particle.color[0] as f32 / 255.0).powf(2.2),
                    (particle.color[1] as f32 / 255.0).powf(2.2),
                    (particle.color[2] as f32 / 255.0).powf(2.2),
                );
                let dynamic = DynamicObject::particle_tile(
                    GeoId::Unknown(key.wrapping_mul(1024).wrapping_add(index as u32)),
                    Self::particle_sprite_tile_id(tile.id),
                    center,
                    basis.1,
                    basis.2,
                    size,
                    size,
                )
                .with_tint(tint)
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
        self.builder_emitters_2d.clear();
        self.builder_emitters_3d.clear();
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

    pub fn find_item_by_sector_id(map: &Map, sector_id: u32) -> Option<&Item> {
        if let Some(item) = map
            .items
            .iter()
            .find(|item| match item.attributes.get("sector_id") {
                Some(Value::UInt(v)) => *v == sector_id,
                Some(Value::Int(v)) if *v >= 0 => *v as u32 == sector_id,
                Some(Value::Int64(v)) if *v >= 0 => *v as u32 == sector_id,
                _ => false,
            })
        {
            return Some(item);
        }

        let group_id = map
            .find_sector(sector_id)?
            .properties
            .get_id("door_group_id")?;
        map.items
            .iter()
            .find(|item| item.attributes.get_id("door_group_id") == Some(group_id))
    }

    fn dungeon_door_mode_from_str(value: &str) -> &'static str {
        match value.trim().to_ascii_lowercase().as_str() {
            "slide_up" | "slide up" => "slide_up",
            "slide_down" | "slide down" => "slide_down",
            "slide_left" | "slide left" => "slide_left",
            "slide_right" | "slide right" => "slide_right",
            "split_sides" | "split sides" => "split_sides",
            _ => "auto",
        }
    }

    fn tile_id_from_source(source: &PixelSource, assets: &Assets) -> Option<Uuid> {
        source.tile_from_tile_list(assets).map(|tile| tile.id)
    }

    fn dynamic_door_tile_id(sector: &crate::Sector, assets: &Assets) -> Option<Uuid> {
        sector
            .properties
            .get_source("jamb_source")
            .and_then(|src| Self::tile_id_from_source(src, assets))
            .or_else(|| {
                sector
                    .properties
                    .get_source("side_source")
                    .and_then(|src| Self::tile_id_from_source(src, assets))
            })
            .or_else(|| {
                sector
                    .properties
                    .get_source("source")
                    .and_then(|src| Self::tile_id_from_source(src, assets))
            })
            .or_else(|| {
                sector
                    .properties
                    .get_source("floor_source")
                    .and_then(|src| Self::tile_id_from_source(src, assets))
            })
            .or_else(|| Uuid::parse_str(DEFAULT_TILE_ID).ok())
    }

    fn item_open_state(item: &Item) -> bool {
        if item.attributes.get("active").is_some() {
            return item.attributes.get_bool_default("active", false);
        }
        if item.attributes.get("blocking").is_some() {
            return !item.attributes.get_bool_default("blocking", true);
        }
        !item.attributes.get_bool_default("visible", true)
    }

    fn apply_dynamic_door_transform(
        points: &mut [Vec3<f32>],
        open_amount: f32,
        mode: &str,
        width: f32,
        height: f32,
        center: Vec3<f32>,
        axis_horizontal: Vec3<f32>,
        axis_normal: Vec3<f32>,
    ) {
        match mode {
            "slide_up" => {
                let offset = Vec3::unit_y() * (height * open_amount);
                for p in points {
                    *p += offset;
                }
            }
            "slide_down" => {
                let offset = Vec3::unit_y() * (height * open_amount);
                for p in points {
                    *p -= offset;
                }
            }
            "slide_left" => {
                let offset = axis_horizontal * (width * open_amount);
                for p in points {
                    *p -= offset;
                }
            }
            "slide_right" => {
                let offset = axis_horizontal * (width * open_amount);
                for p in points {
                    *p += offset;
                }
            }
            "split_sides" => {
                let offset = axis_horizontal * (width * 0.5 * open_amount);
                for p in points {
                    let sign = if (*p - center).dot(axis_horizontal) >= 0.0 {
                        1.0
                    } else {
                        -1.0
                    };
                    *p += offset * sign;
                }
            }
            _ => {
                let offset = axis_normal * (width.max(0.25) * open_amount);
                for p in points {
                    *p += offset;
                }
            }
        }
    }

    fn build_dynamic_door_meshes(
        sector: &crate::Sector,
        map: &Map,
        assets: &Assets,
        open_amount: f32,
        mode: &str,
    ) -> Option<Vec<(Uuid, Vec<DynamicMeshVertex>, Vec<u32>, f32)>> {
        let mut points = sector.vertices_world(map)?;
        if points.len() < 4 {
            return None;
        }
        points.truncate(4);
        let original_points = points.clone();

        let min = points
            .iter()
            .copied()
            .reduce(|a, b| Vec3::new(a.x.min(b.x), a.y.min(b.y), a.z.min(b.z)))?;
        let max = points
            .iter()
            .copied()
            .reduce(|a, b| Vec3::new(a.x.max(b.x), a.y.max(b.y), a.z.max(b.z)))?;
        let center = (min + max) * 0.5;
        let width_x = (max.x - min.x).abs();
        let width_z = (max.z - min.z).abs();
        let axis_horizontal = if width_x >= width_z {
            Vec3::unit_x()
        } else {
            Vec3::unit_z()
        };
        let axis_normal = if width_x >= width_z {
            Vec3::unit_z()
        } else {
            Vec3::unit_x()
        };
        let width = width_x.max(width_z).max(0.001);
        let height = (max.y - min.y).abs().max(0.001);
        Self::apply_dynamic_door_transform(
            &mut points,
            open_amount,
            mode,
            width,
            height,
            center,
            axis_horizontal,
            axis_normal,
        );

        let mut opacity = 1.0f32;
        if open_amount >= 0.999 {
            opacity = 0.0;
        }

        let normal = {
            let a = points[1] - points[0];
            let b = points[2] - points[0];
            let mut n = a.cross(b);
            if n.magnitude_squared() <= 1e-8 {
                n = Vec3::unit_y();
            } else {
                n = n.normalized();
            }
            n
        };

        let Some(surface) = map.get_surface_for_sector_id(sector.id) else {
            let tile_id = Self::dynamic_door_tile_id(sector, assets)?;
            let verts = vec![
                DynamicMeshVertex {
                    position: points[0],
                    uv: Vec2::new(0.0, height),
                    normal,
                },
                DynamicMeshVertex {
                    position: points[1],
                    uv: Vec2::new(0.0, 0.0),
                    normal,
                },
                DynamicMeshVertex {
                    position: points[2],
                    uv: Vec2::new(width, 0.0),
                    normal,
                },
                DynamicMeshVertex {
                    position: points[3],
                    uv: Vec2::new(width, height),
                    normal,
                },
            ];
            return Some(vec![(tile_id, verts, vec![0, 1, 2, 0, 2, 3], opacity)]);
        };

        if let Some(Value::TileOverrides(tile_overrides)) = sector.properties.get("tiles") {
            let original_local: Vec<Vec2<f32>> = original_points
                .iter()
                .map(|p| surface.uv_to_tile_local(surface.world_to_uv(*p), map))
                .collect();
            let min_local = original_local
                .iter()
                .fold(Vec2::new(f32::INFINITY, f32::INFINITY), |acc, p| {
                    Vec2::new(acc.x.min(p.x), acc.y.min(p.y))
                });
            let max_local = original_local
                .iter()
                .fold(Vec2::new(f32::NEG_INFINITY, f32::NEG_INFINITY), |acc, p| {
                    Vec2::new(acc.x.max(p.x), acc.y.max(p.y))
                });
            let base_tile_id = Self::dynamic_door_tile_id(sector, assets)?;

            let mut meshes = Vec::new();
            for ty in min_local.y.floor() as i32..max_local.y.ceil() as i32 {
                for tx in min_local.x.floor() as i32..max_local.x.ceil() as i32 {
                    let x0 = min_local.x.max(tx as f32);
                    let x1 = max_local.x.min(tx as f32 + 1.0);
                    let y0 = min_local.y.max(ty as f32);
                    let y1 = max_local.y.min(ty as f32 + 1.0);
                    if x1 - x0 <= 1e-4 || y1 - y0 <= 1e-4 {
                        continue;
                    }
                    let tile_id = tile_overrides
                        .get(&(tx, ty))
                        .and_then(|source| Self::tile_id_from_source(source, assets))
                        .unwrap_or(base_tile_id);
                    let local_corners = [
                        Vec2::new(x0, y1),
                        Vec2::new(x0, y0),
                        Vec2::new(x1, y0),
                        Vec2::new(x1, y1),
                    ];
                    let mut cell_points: Vec<Vec3<f32>> = local_corners
                        .iter()
                        .map(|local| surface.uv_to_world(surface.tile_local_to_uv(*local, map)))
                        .collect();
                    Self::apply_dynamic_door_transform(
                        &mut cell_points,
                        open_amount,
                        mode,
                        width,
                        height,
                        center,
                        axis_horizontal,
                        axis_normal,
                    );
                    let verts = vec![
                        DynamicMeshVertex {
                            position: cell_points[0],
                            uv: Vec2::new(0.0, y1 - y0),
                            normal,
                        },
                        DynamicMeshVertex {
                            position: cell_points[1],
                            uv: Vec2::new(0.0, 0.0),
                            normal,
                        },
                        DynamicMeshVertex {
                            position: cell_points[2],
                            uv: Vec2::new(x1 - x0, 0.0),
                            normal,
                        },
                        DynamicMeshVertex {
                            position: cell_points[3],
                            uv: Vec2::new(x1 - x0, y1 - y0),
                            normal,
                        },
                    ];
                    meshes.push((tile_id, verts, vec![0, 1, 2, 0, 2, 3], opacity));
                }
            }
            if !meshes.is_empty() {
                return Some(meshes);
            }
        }

        let verts_uv: Vec<[f32; 2]> = original_points
            .iter()
            .map(|p| {
                let uv = surface.world_to_uv(*p);
                [uv.x, uv.y]
            })
            .collect();
        let mut minx = f32::INFINITY;
        let mut miny = f32::INFINITY;
        let mut maxy = f32::NEG_INFINITY;
        for v in &verts_uv {
            minx = minx.min(v[0]);
            miny = miny.min(v[1]);
            maxy = maxy.max(v[1]);
        }
        let wall_like = surface.plane.normal.y.abs() < 0.25;
        let flip_v = wall_like && surface.edit_uv.up.y < 0.0;
        let tile_mode = sector.properties.get_int_default("tile_mode", 1);
        let tex_scale_x = sector
            .properties
            .get_float_default("texture_scale_x", 1.0)
            .max(1e-6);
        let tex_scale_y = sector
            .properties
            .get_float_default("texture_scale_y", 1.0)
            .max(1e-6);
        let sx = (verts_uv
            .iter()
            .map(|v| v[0])
            .fold(f32::NEG_INFINITY, f32::max)
            - minx)
            .max(1e-6);
        let sy = (maxy - miny).max(1e-6);

        let uvs = verts_uv
            .iter()
            .map(|v| {
                let vv = if flip_v { maxy - v[1] } else { v[1] - miny };
                if tile_mode == 0 {
                    Vec2::new((v[0] - minx) / sx, vv / sy)
                } else {
                    Vec2::new((v[0] - minx) / tex_scale_x, vv / tex_scale_y)
                }
            })
            .collect::<Vec<_>>();

        let tile_id = Self::dynamic_door_tile_id(sector, assets)?;
        let verts = vec![
            DynamicMeshVertex {
                position: points[0],
                uv: uvs[0],
                normal,
            },
            DynamicMeshVertex {
                position: points[1],
                uv: uvs[1],
                normal,
            },
            DynamicMeshVertex {
                position: points[2],
                uv: uvs[2],
                normal,
            },
            DynamicMeshVertex {
                position: points[3],
                uv: uvs[3],
                normal,
            },
        ];
        Some(vec![(tile_id, verts, vec![0, 1, 2, 0, 2, 3], opacity)])
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
            base_settings: RenderSettings::default(),

            billboards: FxHashMap::default(),
            billboard_anim_states: FxHashMap::default(),
            door_anim_states: FxHashMap::default(),
            impact_anim_starts: FxHashMap::default(),
            campfire_emitters: FxHashMap::default(),
            tile_emitters_2d: FxHashMap::default(),
            tile_emitters_3d: FxHashMap::default(),
            builder_emitters_2d: FxHashMap::default(),
            builder_emitters_3d: FxHashMap::default(),
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
            pending_particle_steps_2d: 0,
            pending_particle_steps_3d: 0,
            last_dungeon_render_signature: None,
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
            PixelSource::LegacyShapeFXGraphId(id) => {
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
            if let Some(builder_graph_data) = sector.properties.get_str("builder_graph_data") {
                hasher.write(builder_graph_data.as_bytes());
                if let Ok(graph) = BuilderDocument::from_text(builder_graph_data) {
                    for slot in graph.material_slot_names() {
                        let key = format!(
                            "builder_material_{}",
                            Self::normalize_builder_material_key(&slot)
                        );
                        if let Some(Value::Source(source)) = sector.properties.get(&key) {
                            Self::hash_pixel_source(&mut hasher, source);
                        } else {
                            hasher.write_u8(0);
                        }
                    }
                }
            }
        }

        for linedef in &map.linedefs {
            hasher.write_u32(linedef.id);
            if let Some(Value::Source(source)) = linedef.properties.get("row1_source") {
                Self::hash_pixel_source(&mut hasher, source);
            } else {
                hasher.write_u8(0);
            }
            if let Some(builder_graph_data) = linedef.properties.get_str("builder_graph_data") {
                hasher.write(builder_graph_data.as_bytes());
                if let Ok(graph) = BuilderDocument::from_text(builder_graph_data) {
                    for slot in graph.material_slot_names() {
                        let key = format!(
                            "builder_material_{}",
                            Self::normalize_builder_material_key(&slot)
                        );
                        if let Some(Value::Source(source)) = linedef.properties.get(&key) {
                            Self::hash_pixel_source(&mut hasher, source);
                        } else {
                            hasher.write_u8(0);
                        }
                    }
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
            if let Some(builder_graph_data) = vertex.properties.get_str("builder_graph_data") {
                hasher.write(builder_graph_data.as_bytes());
                if let Ok(graph) = BuilderDocument::from_text(builder_graph_data) {
                    for slot in graph.material_slot_names() {
                        let key = format!(
                            "builder_material_{}",
                            Self::normalize_builder_material_key(&slot)
                        );
                        if let Some(Value::Source(source)) = vertex.properties.get(&key) {
                            Self::hash_pixel_source(&mut hasher, source);
                        } else {
                            hasher.write_u8(0);
                        }
                    }
                }
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
            if let Some(builder_graph_data) = sector.properties.get_str("builder_graph_data") {
                hasher.write(builder_graph_data.as_bytes());
                if let Ok(graph) = BuilderDocument::from_text(builder_graph_data) {
                    for slot in graph.material_slot_names() {
                        let key = format!(
                            "builder_material_{}",
                            Self::normalize_builder_material_key(&slot)
                        );
                        if let Some(Value::Source(source)) = sector.properties.get(&key) {
                            Self::hash_pixel_source(&mut hasher, source);
                        } else {
                            hasher.write_u8(0);
                        }
                    }
                }
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
            if let Some(builder_graph_data) = linedef.properties.get_str("builder_graph_data") {
                hasher.write(builder_graph_data.as_bytes());
                if let Ok(graph) = BuilderDocument::from_text(builder_graph_data) {
                    for slot in graph.material_slot_names() {
                        let key = format!(
                            "builder_material_{}",
                            Self::normalize_builder_material_key(&slot)
                        );
                        if let Some(Value::Source(source)) = linedef.properties.get(&key) {
                            Self::hash_pixel_source(&mut hasher, source);
                        } else {
                            hasher.write_u8(0);
                        }
                    }
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
            if let Some(builder_graph_data) = vertex.properties.get_str("builder_graph_data") {
                hasher.write(builder_graph_data.as_bytes());
                if let Ok(graph) = BuilderDocument::from_text(builder_graph_data) {
                    for slot in graph.material_slot_names() {
                        let key = format!(
                            "builder_material_{}",
                            Self::normalize_builder_material_key(&slot)
                        );
                        if let Some(Value::Source(source)) = vertex.properties.get(&key) {
                            Self::hash_pixel_source(&mut hasher, source);
                        } else {
                            hasher.write_u8(0);
                        }
                    }
                }
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
                let sprite =
                    Self::build_particle_sprite_texture(emitter.color, emitter.color_ramp.as_ref());
                self.vm.execute(Atom::AddTile {
                    id: Self::particle_sprite_tile_id(*id),
                    width: sprite.width as u32,
                    height: sprite.height as u32,
                    frames: vec![sprite.data],
                    material_frames: sprite.data_ext.clone().map(|ext| vec![ext]),
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
                id: self.gray,
                color: [138, 138, 138, 220],
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
        let has_active_builder_particles = !self.builder_emitters_2d.is_empty();
        if self.dynamics_ready_2d
            && self.last_dynamics_hash_2d == Some(current_hash)
            && !has_active_tile_particles
            && !has_active_builder_particles
        {
            return;
        }
        self.last_dynamics_hash_2d = Some(current_hash);
        let particle_steps = std::mem::take(&mut self.pending_particle_steps_2d);

        self.vm.execute(Atom::ClearDynamics);
        self.vm.execute(Atom::ClearLights);
        let _has_tile_particles = self.rebuild_tile_particles_2d(map, assets, particle_steps);
        let _has_builder_particles = self.rebuild_builder_particles_2d(map, assets, particle_steps);
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
                let size_2d = entity
                    .attributes
                    .get_float_default("size_2d", 1.0)
                    .max(0.01);
                // 2D avatar billboards are center-anchored. Shift the center upward so
                // increasing size_2d keeps the avatar inside the tile instead of growing
                // downward out of the cell.
                let avatar_pos_2d = Vec2::new(pos.x, pos.y - (size_2d - 1.0) * 0.5);
                let has_avatar_binding = AvatarRuntimeBuilder::has_avatar_binding(entity);
                let keep_cached_avatar = self.avatar_builder.has_cached_avatar(geo_id);
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
                        let dynamic = DynamicObject::billboard_avatar_2d(
                            geo_id,
                            avatar_pos_2d,
                            size_2d,
                            size_2d,
                        )
                        .with_layer(20);
                        self.vm.execute(Atom::AddDynamic { object: dynamic });
                        continue;
                    }
                }

                if has_avatar_binding {
                    if keep_cached_avatar {
                        active_avatar_geo.insert(geo_id);
                        let dynamic = DynamicObject::billboard_avatar_2d(
                            geo_id,
                            avatar_pos_2d,
                            size_2d,
                            size_2d,
                        )
                        .with_layer(20);
                        self.vm.execute(Atom::AddDynamic { object: dynamic });
                    }
                    continue;
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
        let has_active_builder_particles = !self.builder_emitters_3d.is_empty();
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
            && !has_active_builder_particles
        {
            return;
        }
        self.last_dynamics_hash_3d = Some(current_hash);
        let particle_steps = std::mem::take(&mut self.pending_particle_steps_3d);
        self.vm.execute(Atom::ClearDynamics);
        self.vm.execute(Atom::ClearLights);
        self.add_sector_campfire_lights(map);
        let _has_campfire_particles =
            self.rebuild_campfire_particles(map, camera, assets, particle_steps);
        let _has_tile_particles =
            self.rebuild_tile_particles_3d(map, camera, assets, particle_steps);
        let _has_builder_particles =
            self.rebuild_builder_particles_3d(map, camera, assets, particle_steps);
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
        const DUNGEON_DOOR_ANIMATION_DURATION_S: f32 = 0.30;

        // Drop stale animation states for billboards that vanished with chunk updates.
        self.billboard_anim_states
            .retain(|geo_id, _| self.billboards.contains_key(geo_id));

        let mut active_door_geo: FxHashSet<GeoId> = FxHashSet::default();
        for sector in &map.sectors {
            if sector
                .properties
                .get_str_default("generated_by", String::new())
                != "dungeon_tool"
            {
                continue;
            }
            if sector
                .properties
                .get_str_default("dungeon_part", String::new())
                != "door_panel"
            {
                continue;
            }

            let geo_id = GeoId::Sector(sector.id);
            active_door_geo.insert(geo_id);

            let Some(item) = Self::find_item_by_sector_id(map, sector.id) else {
                continue;
            };
            let desired_open = if Self::item_open_state(item) {
                1.0
            } else {
                0.0
            };
            let state = self
                .door_anim_states
                .entry(geo_id)
                .or_insert_with(|| DoorAnimState::new(desired_open, self.frame_counter));
            if (desired_open - state.target_open).abs() > f32::EPSILON {
                let current_open = state.open_amount(
                    self.frame_counter,
                    self.render_fps,
                    DUNGEON_DOOR_ANIMATION_DURATION_S,
                );
                *state = DoorAnimState {
                    start_open: current_open,
                    target_open: desired_open,
                    start_frame: self.frame_counter,
                };
            }
            let open_amount = state.open_amount(
                self.frame_counter,
                self.render_fps,
                DUNGEON_DOOR_ANIMATION_DURATION_S,
            );
            if (open_amount - state.target_open).abs() <= 1e-3 {
                state.start_open = state.target_open;
                state.start_frame = self.frame_counter;
            }

            let mut mode = Self::dungeon_door_mode_from_str(
                &sector
                    .properties
                    .get_str_default("dungeon_door_mode", "auto".to_string()),
            );
            if mode == "split_sides" {
                mode = match sector
                    .properties
                    .get_str_default("dungeon_door_leaf", String::new())
                    .as_str()
                {
                    "left" => "slide_left",
                    "right" => "slide_right",
                    _ => "split_sides",
                };
            }
            let Some(meshes) =
                Self::build_dynamic_door_meshes(sector, map, assets, open_amount, mode)
            else {
                continue;
            };
            for (tile_id, verts, indices, opacity) in meshes {
                let dynamic =
                    DynamicObject::mesh(geo_id, tile_id, verts, indices).with_opacity(opacity);
                self.vm.execute(Atom::AddDynamic { object: dynamic });
            }
        }
        self.door_anim_states
            .retain(|geo_id, _| active_door_geo.contains(geo_id));

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
