//! Ruleset MZ appearances: one editable recipe, directional 2D sprites and live 3D meshes.
use crate::{Assets, Entity, Texture};
use denrim_materials::geometry::{Document, Evaluation, Mesh, Operation, Value as GeoValue};
use rustc_hash::{FxHashMap, FxHashSet, FxHasher};
use scenevm::GeoId;
#[cfg(feature = "graphics")]
use scenevm::{Atom, DynamicMeshVertex, DynamicObject, SceneVM};
use std::{
    hash::{Hash, Hasher},
    sync::{Arc, Mutex},
};
use theframework::prelude::{ThePalette, Uuid};
pub mod effects;
#[cfg(feature = "graphics")]
mod meshes;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod preparation;
mod weapon;
pub mod wearable;
#[cfg(any(feature = "graphics", test))]
use vek::Vec2;
use vek::Vec3;
pub use weapon::attack_duration;
pub use wearable::Wearable;

pub const SPRITE_SIZE: usize = 64;
const FPS: f32 = 16.;
pub fn fingerprint(value: &impl Hash) -> u64 {
    let mut h = FxHasher::default();
    value.hash(&mut h);
    h.finish()
}
struct Material {
    id: Uuid,
    visible: bool,
    texture: Texture,
    packed: Vec<u8>,
    frames: Vec<(Uuid, Texture, Vec<u8>)>,
    duration: f32,
}
impl Material {
    fn frame_index(&self, seconds: f32) -> usize {
        if self.frames.is_empty() || !seconds.is_finite() {
            0
        } else {
            (seconds.max(0.).rem_euclid(self.duration.max(0.001)) / self.duration.max(0.001)
                * self.frames.len() as f32)
                .floor() as usize
                % self.frames.len()
        }
    }
    fn at(&self, seconds: f32) -> (Uuid, &Texture, &[u8]) {
        self.frames
            .get(self.frame_index(seconds))
            .map(|(id, t, p)| (*id, t, p.as_slice()))
            .unwrap_or((self.id, &self.texture, &self.packed))
    }
}
struct Compiled {
    evaluation: Evaluation,
    motion_names: FxHashMap<String, String>,
    materials: Vec<Material>,
    height: f32,
    floor: f32,
    extent: f32,
}
pub struct Appearance {
    pub source: String,
    pub signature: u64,
    pub has_effects: bool,
    document: Document,
    pub(crate) state_on: Option<Arc<Appearance>>,
    pub(crate) footprint: Option<denrim_materials::geometry::footprint::Shape<[f32; 3]>>,
    pub(crate) bounds: ([f32; 3], [f32; 3]),
    pub(crate) connectors: Vec<denrim_materials::geometry::connectors::Connector>,
    equipment: Vec<(String, String, String)>,
    compiled: Mutex<FxHashMap<u64, Arc<Compiled>>>,
    sprites: Mutex<FxHashMap<(u64, String, u32, i32, u64, bool), crate::isometric::AvatarSprite>>,
    last_ready: Mutex<FxHashMap<(Uuid, u32), u64>>,
    preparing: Mutex<FxHashSet<((Uuid, u32), u64)>>,
}
struct VariantPlan {
    mask: u64,
    key: u64,
    replacements: Vec<(usize, Arc<Appearance>, bool)>,
    worn: Vec<Wearable>,
    profile: weapon::Profile,
}
impl Appearance {
    /// Whether the exact requested outfit is ready, without compiling it.
    pub fn equipment_is_prepared(&self, entity: &Entity, assets: &Assets) -> bool {
        self.compiled
            .lock()
            .unwrap()
            .contains_key(&self.variant_plan(entity, assets).key)
    }

    pub fn new(
        source: &str,
        equipment: &[(&str, &str, &str)],
        signature: u64,
    ) -> Result<Self, String> {
        let mut document = Document::parse(source).map_err(|e| e.to_string())?;
        let state_on = if document.has_states() {
            let on = serde_json::to_string(&document.for_state(true)).map_err(|e| e.to_string())?;
            document = document.for_state(false);
            Some(Arc::new(Self::new(&on, equipment, fingerprint(&(signature, true)))?))
        } else { None };
        let mesh = document.evaluate().map_err(|e| e.to_string())?.mesh;
        let has_effects = !mesh.effects.is_empty();
        let bounds = mesh.bounds();
        Ok(Self {
            source: source.into(),
            signature,
            has_effects,
            document,
            state_on,
            footprint: mesh.footprint,
            connectors: mesh.connectors,
            bounds,
            equipment: equipment
                .iter()
                .map(|(a, b, c)| (a.to_string(), b.to_string(), c.to_string()))
                .collect(),
            compiled: Default::default(),
            sprites: Default::default(),
            last_ready: Default::default(),
            preparing: Default::default(),
        })
    }
    fn equipment_mask(&self, entity: &Entity, assets: &Assets) -> u64 {
        let mut mask = 0;
        for (i, (_, slot, category)) in self.equipment.iter().take(64).enumerate() {
            if Self::equipped_for_attachment(entity, slot, assets).is_some_and(|item| {
                let name = format!(
                    "{} {} {} {}",
                    item.item_type,
                    item.attributes.get_str("name").unwrap_or(""),
                    item.attributes.get_str("ruleset_id").unwrap_or(""),
                    item.attributes.get_str("ruleset_path").unwrap_or("")
                );
                name.to_ascii_lowercase().contains(category)
            }) {
                mask |= 1 << i;
            }
        }
        mask
    }
    fn compile(&self, mask: u64, assets: &Assets) -> Result<Arc<Compiled>, String> {
        self.compile_variant(mask, assets, &[], &[], &weapon::Profile::default(), mask)
    }
    fn equipped_for_attachment<'a>(
        entity: &'a Entity,
        slot: &str,
        assets: &Assets,
    ) -> Option<&'a crate::Item> {
        entity.equipped.get(slot).or_else(|| {
            let rules = assets.rules_table()?;
            let anchors = rules.get("equipment")?.get("avatar_anchors")?.as_table()?;
            entity.equipped.iter().find_map(|(equipped_slot, item)| {
                (anchors.get(equipped_slot).and_then(toml::Value::as_str) == Some(slot))
                    .then_some(item)
            })
        })
    }
    fn variant_plan(&self, entity: &Entity, assets: &Assets) -> VariantPlan {
        let mut mask = self.equipment_mask(entity, assets);
        let mut replacements = vec![];
        let mut signatures = vec![];
        let profile = weapon::Profile::for_entity(entity);
        for (i, (_, slot, _)) in self.equipment.iter().enumerate().take(64) {
            if profile.two_handed && slot == "off_hand" {
                mask &= !(1 << i);
                continue;
            }
            if let Some(model) = Self::equipped_for_attachment(entity, slot, assets)
                .and_then(|item| Runtime::resolve_item(item, assets))
            {
                mask |= 1 << i;
                let inactive = Self::equipped_for_attachment(entity, slot, assets)
                    .is_some_and(|item| !Runtime::item_effects_enabled(item));
                replacements.push((i, model.clone(), inactive));
                signatures.push((i, model.signature, inactive));
            }
        }
        // Sort slots to make overlapping authored bindings deterministic.
        let mut equipped = entity.equipped.iter().collect::<Vec<_>>();
        equipped.sort_by_key(|(slot, _)| *slot);
        let worn = equipped
            .into_iter()
            .filter_map(|(_, item)| Runtime::resolve_wearable(item, assets))
            .collect::<Vec<_>>();
        let key = if signatures.is_empty() && worn.is_empty() {
            mask
        } else {
            fingerprint(&(
                mask,
                signatures,
                &profile,
                worn.iter().map(Wearable::signature).collect::<Vec<_>>(),
                worn.iter()
                    .filter_map(|w| w.geometry.as_ref())
                    .filter_map(|source| Runtime::worn_model(source, assets))
                    .map(|a| a.signature)
                    .collect::<Vec<_>>(),
            ))
        };
        VariantPlan {
            mask,
            key,
            replacements,
            worn,
            profile,
        }
    }

    fn compiled_for(
        &self,
        entity: &Entity,
        assets: &Assets,
    ) -> Result<(Arc<Compiled>, u64), String> {
        let plan = self.variant_plan(entity, assets);
        if let Some(cached) = self.compiled.lock().unwrap().get(&plan.key).cloned() {
            if entity.id != 0 {
                self.last_ready
                    .lock()
                    .unwrap()
                    .insert((entity.creator_id, entity.id), plan.key);
            }
            return Ok((cached, plan.key));
        }
        if self
            .preparing
            .lock()
            .unwrap()
            .contains(&((entity.creator_id, entity.id), plan.key))
        {
            let previous = self
                .last_ready
                .lock()
                .unwrap()
                .get(&(entity.creator_id, entity.id))
                .copied();
            if let Some(key) = previous {
                if let Some(cached) = self.compiled.lock().unwrap().get(&key).cloned() {
                    return Ok((cached, key));
                }
            }
        }
        // First appearance/editor previews remain exact. Later equipment changes
        // retain the ready variant while the update-side queue prepares a new one.
        let result = self.evaluate_plan(&plan, assets)?;
        self.publish_variant(plan.key, result.clone());
        if entity.id != 0 {
            self.last_ready
                .lock()
                .unwrap()
                .insert((entity.creator_id, entity.id), plan.key);
        }
        Ok((result, plan.key))
    }

    fn evaluate_plan(&self, plan: &VariantPlan, assets: &Assets) -> Result<Arc<Compiled>, String> {
        let replacements = plan
            .replacements
            .iter()
            .map(|(i, model, inactive)| {
                let mut doc = model.document.clone();
                if *inactive {
                    disable_effects(&mut doc);
                }
                (*i, doc)
            })
            .collect::<Vec<_>>();
        self.build_variant(plan.mask, assets, &replacements, &plan.worn, &plan.profile)
    }

    fn publish_variant(&self, key: u64, result: Arc<Compiled>) {
        let mut cache = self.compiled.lock().unwrap();
        if cache.len() >= 64 && !cache.contains_key(&key) {
            let active = self.last_ready.lock().unwrap();
            if let Some(old) = cache
                .keys()
                .find(|k| !active.values().any(|v| v == *k))
                .copied()
            {
                cache.remove(&old);
            }
        }
        cache.insert(key, result);
    }

    fn compile_variant(
        &self,
        mask: u64,
        assets: &Assets,
        replacements: &[(usize, Document)],
        worn: &[Wearable],
        profile: &weapon::Profile,
        key: u64,
    ) -> Result<Arc<Compiled>, String> {
        if let Some(cached) = self.compiled.lock().unwrap().get(&key).cloned() {
            return Ok(cached);
        }
        let result = self.build_variant(mask, assets, replacements, worn, profile)?;
        self.publish_variant(key, result.clone());
        Ok(result)
    }

    fn build_variant(
        &self,
        mask: u64,
        assets: &Assets,
        replacements: &[(usize, Document)],
        worn: &[Wearable],
        profile: &weapon::Profile,
    ) -> Result<Arc<Compiled>, String> {
        let mut doc = self.document.clone();
        for (index, model) in replacements {
            let attachment = &self.equipment[*index].0;
            if let Some(row) = doc
                .rows
                .iter_mut()
                .find(|r| r.tiles.iter().any(|t| &t.id == attachment))
            {
                for tile in &mut row.tiles {
                    match &mut tile.op {
                        Operation::Asset { document, .. } | Operation::Group { document, .. } => {
                            **document = model.clone();
                            break;
                        }
                        _ => {}
                    }
                }
            }
        }
        for (i, (operation, _, _)) in self.equipment.iter().take(64).enumerate() {
            for tile in doc.rows.iter_mut().flat_map(|r| &mut r.tiles) {
                if &tile.id == operation {
                    if let Operation::Attach { enabled, .. } = &mut tile.op {
                        *enabled &= mask & (1 << i) != 0;
                    }
                }
            }
        }
        if !profile.joint.is_empty() {
            for (operation, slot, _) in &self.equipment {
                if slot == "main_hand" {
                    for tile in doc.rows.iter_mut().flat_map(|r| &mut r.tiles) {
                        if &tile.id == operation
                            && let Operation::Attach { joint, .. } = &mut tile.op
                        {
                            *joint = profile.joint.clone();
                        }
                    }
                }
            }
        }
        let mut evaluation = doc.evaluate().map_err(|e| e.to_string())?;
        // Motion rows may refer to the body group rather than the assembled model.
        if let Some(rig) = &evaluation.mesh.rig {
            for value in evaluation.rows.values_mut() {
                if let GeoValue::Motion(m) = value {
                    if m.rig.joints.iter().all(|j| rig.index(&j.id).is_ok()) {
                        m.rig = rig.clone();
                    }
                }
            }
            // A rigged item contributes keyed motion to the matching character
            // clip. Joint IDs survive Attach, including its local bind frames.
            for (_, model) in replacements {
                let source = model.evaluate().map_err(|e| e.to_string())?;
                for (id, value) in source.rows {
                    if let GeoValue::Motion(item_motion) = value {
                        if let Some(GeoValue::Motion(body_motion)) = evaluation.rows.get_mut(&id) {
                            for track in item_motion.tracks {
                                if rig.index(&track.joint).is_ok() {
                                    body_motion.tracks.retain(|t| t.joint != track.joint);
                                    body_motion.tracks.push(track);
                                }
                            }
                            body_motion.events.extend(item_motion.events);
                            body_motion.events.sort_by(|a, b| {
                                a.time.total_cmp(&b.time).then(a.name.cmp(&b.name))
                            });
                            body_motion
                                .events
                                .dedup_by(|a, b| a.time == b.time && a.name == b.name);
                            body_motion.validate().map_err(|e| e.to_string())?;
                        }
                    }
                }
            }
        }
        let body = doc
            .rows
            .iter()
            .flat_map(|r| &r.tiles)
            .find_map(|t| {
                if let Operation::Motion { rig, .. } = &t.op {
                    evaluation.rows.get(rig)
                } else {
                    None
                }
            })
            .and_then(|v| {
                if let GeoValue::Mesh(mesh) = v {
                    Some(mesh)
                } else {
                    None
                }
            })
            .unwrap_or(&evaluation.mesh);
        let (lo, hi) = body.bounds();
        let height = (hi[1] - lo[1]).max(0.01);
        // Stable framing across clips, headings and equipment variants.
        let extent = if body.rig.is_some() {
            height * 0.70
        } else {
            // Flat coins and broad props need horizontal room in model previews.
            // The diagonal bounds also cover every heading without changing scale.
            height.max((hi[0] - lo[0]).hypot(hi[2] - lo[2])) * 0.70
        };
        let wearer = doc.wearer_body().map_err(|e| e.to_string())?;
        // Fitted material layers retain the recipe's UVs and skin weights. Body
        // framing and sockets above are measured before garment padding.
        for garment in worn {
            if let Some(source) = &garment.geometry {
                let model = Runtime::worn_model(source, assets)
                    .ok_or_else(|| format!("Missing worn geometry {source}"))?;
                let shell = model
                    .document
                    .with_wearer(&wearer)
                    .evaluate()
                    .map_err(|e| e.to_string())?
                    .mesh;
                evaluation
                    .mesh
                    .replace_parts(shell)
                    .map_err(|e| e.to_string())?;
            }
            for part in &mut evaluation.mesh.parts {
                if let Some(source) = garment.materials.get(&part.id) {
                    part.texture = Some(denrim_materials::geometry::Texture {
                        source: source.clone(),
                        bundle: None,
                        size: 64,
                    });
                    for vertex in &mut part.vertices {
                        for k in 0..3 {
                            if garment.geometry.is_none() {
                                vertex.position[k] += vertex.normal[k] * garment.padding;
                            }
                        }
                    }
                }
            }
        }
        let mut materials = vec![];
        for part in &evaluation.mesh.parts {
            let surface = part.surface.as_ref();
            let color = palette_color(&assets.palette, surface.map_or(0, |s| s.palette));
            let texture = part
                .texture
                .as_ref()
                .and_then(|t| {
                    if assets.ruleset_materials.contains_key(&t.source) {
                        assets.ruleset_materials.get(&t.source).map(|texture| {
                            if texture.width != t.size as usize {
                                texture.resized(t.size as usize, t.size as usize)
                            } else {
                                texture.clone()
                            }
                        })
                    } else if let Some(path) = t.source.strip_prefix("ruleset:icon/") {
                        let (item, state) = path.rsplit_once('/')?;
                        assets
                            .item_icons
                            .get(&format!("{item}:{state}"))
                            .and_then(|v| v.first())
                            .cloned()
                            .or_else(|| assets.textures.get(item).cloned())
                    } else {
                        Uuid::parse_str(&t.source)
                            .ok()
                            .and_then(|id| assets.tiles.get(&id))
                            .and_then(|t| t.textures.first())
                            .cloned()
                    }
                })
                .unwrap_or_else(|| Texture::new(color.to_vec(), 1, 1));
            let r = (surface.map_or(0.65, |s| s.roughness) * 15.).round() as u8;
            let m = (surface.map_or(0., |s| s.metallic) * 15.).round() as u8;
            let rm = r | (m << 4);
            let packed = texture.data_ext.clone().unwrap_or_else(|| {
                [if rm == 0xfe { 0xff } else { rm }, 15, 128, 128]
                    .repeat(texture.width * texture.height)
            });
            let hash = fingerprint(&(texture.width, texture.height, &texture.data, &packed));
            let id = Uuid::from_u128((0x4d5a5f4d41544552u128 << 64) | hash as u128);
            let animation = part
                .texture
                .as_ref()
                .and_then(|t| assets.material_animations.get(&t.source));
            let duration = animation.map_or(0.25, |(_, d)| *d);
            let frames = animation
                .filter(|(f, _)| f.len() > 1)
                .map(|(frames, _)| {
                    frames
                        .iter()
                        .map(|t| {
                            let t = t.resized(texture.width, texture.height);
                            let p = t.data_ext.clone().unwrap_or_else(|| {
                                [if rm == 0xfe { 0xff } else { rm }, 15, 128, 128]
                                    .repeat(t.width * t.height)
                            });
                            let hash = fingerprint(&(t.width, t.height, &t.data, &p));
                            (
                                Uuid::from_u128((0x4d5a5f4d41544552u128 << 64) | hash as u128),
                                t,
                                p,
                            )
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            materials.push(Material {
                id,
                visible: texture.data.chunks_exact(4).any(|p| p[3] != 0)
                    || frames
                        .iter()
                        .any(|(_, t, _)| t.data.chunks_exact(4).any(|p| p[3] != 0)),
                frames,
                duration,
                texture,
                packed,
            });
        }
        let mut motion_names = FxHashMap::default();
        for row in &doc.rows {
            if matches!(evaluation.rows.get(&row.id), Some(GeoValue::Motion(_))) {
                motion_names.insert(row.id.to_ascii_lowercase(), row.id.clone());
                motion_names
                    .entry(row.label.trim().to_ascii_lowercase())
                    .or_insert(row.id.clone());
            }
        }
        for (alias, selected) in [
            ("idle", &profile.idle),
            ("walk", &profile.walk),
            ("attack", &profile.attack),
        ] {
            if let Some(id) = motion_names.get(selected).cloned() {
                motion_names.insert(alias.into(), id);
            }
        }
        let result = Arc::new(Compiled {
            evaluation,
            motion_names,
            materials,
            height,
            floor: lo[1],
            extent,
        });
        Ok(result)
    }
    fn motion<'a>(
        c: &'a Compiled,
        animation: &str,
    ) -> Option<&'a denrim_materials::geometry::rig::Motion> {
        let id = c
            .motion_names
            .get(animation)
            .or_else(|| c.motion_names.get("idle"))?;
        if let Some(GeoValue::Motion(m)) = c.evaluation.rows.get(id) {
            Some(m)
        } else {
            None
        }
    }
    fn animation_seconds(c: &Compiled, entity: &Entity, animation: &str, seconds: f32) -> f32 {
        if animation == "attack" {
            if let Some(m) = Self::motion(c, animation) {
                return seconds * m.duration / attack_duration(entity);
            }
        }
        seconds
    }
    #[cfg(test)]
    fn pose(c: &Compiled, animation: &str, seconds: f32) -> Result<Mesh, String> {
        c.evaluation
            .mesh
            .posed(Self::motion(c, animation), seconds)
            .map(|p| p.mesh)
            .map_err(|e| e.to_string())
    }
    /// Runtime clip length, including the selected weapon's attack timing.
    pub fn clip_duration(
        &self,
        entity: &Entity,
        assets: &Assets,
        animation: &str,
    ) -> Result<f32, String> {
        let (compiled, _) = self.compiled_for(entity, assets)?;
        Ok(if animation == "attack" {
            attack_duration(entity)
        } else {
            Self::motion(&compiled, animation).map_or(1.0, |motion| motion.duration)
        }
        .max(0.001))
    }
    pub fn sprite(
        &self,
        entity: &Entity,
        assets: &Assets,
        animation: &str,
        seconds: f32,
    ) -> Result<Arc<Vec<u8>>, String> {
        self.sprite_at(entity, assets, animation, seconds, seconds)
    }
    pub fn sprite_at(
        &self,
        entity: &Entity,
        assets: &Assets,
        animation: &str,
        seconds: f32,
        surface_seconds: f32,
    ) -> Result<Arc<Vec<u8>>, String> {
        self.sprite_with_pose(entity, assets, animation, seconds, surface_seconds, None)
    }
    pub fn sprite_with_pose(
        &self,
        entity: &Entity,
        assets: &Assets,
        animation: &str,
        seconds: f32,
        surface_seconds: f32,
        joints: Option<&[denrim_materials::geometry::rig::Pose]>,
    ) -> Result<Arc<Vec<u8>>, String> {
        self.sprite_camera(
            entity,
            assets,
            animation,
            seconds,
            surface_seconds,
            joints,
            false,
        )
        .map(|f| f.pixels)
    }
    pub fn sprite_isometric(
        &self,
        entity: &Entity,
        assets: &Assets,
        animation: &str,
        seconds: f32,
        surface_seconds: f32,
        joints: Option<&[denrim_materials::geometry::rig::Pose]>,
    ) -> Result<Arc<Vec<u8>>, String> {
        self.sprite_camera(
            entity,
            assets,
            animation,
            seconds,
            surface_seconds,
            joints,
            true,
        )
        .map(|f| f.pixels)
    }
    fn sprite_camera(
        &self,
        entity: &Entity,
        assets: &Assets,
        animation: &str,
        seconds: f32,
        surface_seconds: f32,
        joints: Option<&[denrim_materials::geometry::rig::Pose]>,
        isometric: bool,
    ) -> Result<crate::isometric::AvatarSprite, String> {
        let (c, mask) = self.compiled_for(entity, assets)?;
        let seconds = Self::animation_seconds(&c, entity, animation, seconds);
        let clip = Self::motion(&c, animation);
        let seconds = clip.map_or(seconds, |m| {
            if m.looping {
                seconds.rem_euclid(m.duration)
            } else {
                seconds.min(m.duration)
            }
        });
        let complete = clip.is_some_and(|m| !m.looping && seconds + 0.000001 >= m.duration);
        let frame = if complete {
            (seconds * FPS).ceil()
        } else {
            (seconds * FPS).floor()
        } as u32;
        let yaw = entity.attributes.get_float("appearance_yaw").unwrap_or_else(|| (entity
            .orientation
            .x
            .atan2(entity.orientation.y)
            .to_degrees()
            / 45.)
            .round() * 45.).rem_euclid(360.);
        let textures = fingerprint(&(
            c.materials
                .iter()
                .map(|m| m.frame_index(surface_seconds))
                .collect::<Vec<_>>(),
            joints.map(|j| {
                j.iter()
                    .flat_map(|p| p.translation.iter().chain(&p.rotation))
                    .map(|f| f.to_bits())
                    .collect::<Vec<_>>()
            }),
        ));
        let key = (
            mask,
            animation.into(),
            frame,
            yaw.to_bits() as i32,
            textures,
            isometric,
        );
        if let Some(rgba) = self.sprites.lock().unwrap().get(&key) {
            return Ok(rgba.clone());
        }
        // A duration need not be a multiple of the sprite frame interval. Give
        // its final key a separate cache entry, so release/reload is never lost.
        let sample = clip
            .filter(|m| !m.looping)
            .map_or(frame as f32 / FPS, |m| (frame as f32 / FPS).min(m.duration));
        let mesh = c
            .evaluation
            .mesh
            .posed_with_joints(Self::motion(&c, animation), sample, joints)
            .map_err(|e| e.to_string())?
            .mesh;
        let pixels = raster_sprite_frame(
            &mesh,
            &c,
            yaw,
            &assets.palette,
            surface_seconds,
            isometric,
        );
        let mut cache = self.sprites.lock().unwrap();
        if cache.len() >= 512 {
            cache.clear();
        }
        cache.insert(key, pixels.clone());
        Ok(pixels)
    }
    /// Resolve effect attachment positions from exactly the same posed mesh as rendering.
    pub fn effects(
        &self,
        entity: &Entity,
        assets: &Assets,
        feet: Vec3<f32>,
        size: f32,
        animation: &str,
        seconds: f32,
        ground_flat: bool,
        two_d: bool,
    ) -> Result<Vec<effects::Source>, String> {
        self.effects_with_pose(
            entity,
            assets,
            feet,
            size,
            animation,
            seconds,
            ground_flat,
            two_d,
            None,
        )
    }
    pub fn effects_with_pose(
        &self,
        entity: &Entity,
        assets: &Assets,
        feet: Vec3<f32>,
        size: f32,
        animation: &str,
        seconds: f32,
        ground_flat: bool,
        two_d: bool,
        joints: Option<&[denrim_materials::geometry::rig::Pose]>,
    ) -> Result<Vec<effects::Source>, String> {
        let (c, _) = self.compiled_for(entity, assets)?;
        if c.evaluation.mesh.effects.is_empty() {
            return Ok(vec![]);
        }
        let mut sample = Self::animation_seconds(&c, entity, animation, seconds);
        if two_d {
            let clip = Self::motion(&c, animation);
            sample = clip.map_or(sample, |m| {
                if m.looping {
                    sample.rem_euclid(m.duration)
                } else {
                    sample.min(m.duration)
                }
            });
            let complete = clip.is_some_and(|m| !m.looping && sample + 0.000001 >= m.duration);
            let frame = if complete {
                (sample * FPS).ceil()
            } else {
                (sample * FPS).floor()
            };
            sample = clip
                .filter(|m| !m.looping)
                .map_or(frame / FPS, |m| (frame / FPS).min(m.duration));
        }
        let (effects, flat_floor, center) = if ground_flat && !two_d {
            let mesh = c
                .evaluation
                .mesh
                .posed_with_joints(Self::motion(&c, animation), sample, joints)
                .map_err(|e| e.to_string())?
                .mesh;
            let floor = mesh
                .parts
                .iter()
                .flat_map(|p| &p.vertices)
                .map(|v| v.position[2])
                .fold(f32::INFINITY, f32::min);
            let (lo, hi) = mesh.bounds();
            (
                mesh.effects,
                floor,
                [(lo[0] + hi[0]) * 0.5, -(lo[1] + hi[1]) * 0.5],
            )
        } else {
            let effects = c
                .evaluation
                .mesh
                .posed_effects_with_joints(Self::motion(&c, animation), sample, joints)
                .map_err(|e| e.to_string())?;
            (effects, 0., [0.; 2])
        };
        let yaw = entity.orientation.x.atan2(entity.orientation.y);
        let yaw = if two_d {
            (yaw.to_degrees() / 45.).round() * 45f32.to_radians()
        } else {
            yaw
        };
        let (s, co) = yaw.sin_cos();
        Ok(effects
            .into_iter()
            .filter(|e| e.definition.rate > 0. || e.definition.light > 0.)
            .map(|effect| {
                let p = effect.definition.position;
                let (position, scale) = if two_d {
                    let x = co * p[0] + s * p[2];
                    let z = -s * p[0] + co * p[2];
                    (
                        feet + Vec3::new(
                            x / (2. * c.extent) * size,
                            0.,
                            (-(p[1] - c.floor - c.height * 0.5) * 0.94 + z * 0.34)
                                / (2. * c.extent)
                                * size,
                        ),
                        size / (2. * c.extent),
                    )
                } else {
                    let p = if ground_flat {
                        [p[0] - center[0], p[2] - flat_floor, -p[1] - center[1]]
                    } else {
                        [p[0], p[1] - c.floor, p[2]]
                    };
                    let scale = size / c.height;
                    (
                        feet + Vec3::new(co * p[0] + s * p[2], p[1], -s * p[0] + co * p[2]) * scale,
                        scale,
                    )
                };
                effects::Source {
                    id: effect.id,
                    definition: effect.definition,
                    position,
                    scale,
                }
            })
            .collect())
    }
    pub fn has_entity_effects(&self, entity: &Entity, assets: &Assets) -> bool {
        self.compiled_for(entity, assets).is_ok_and(|(c, _)| {
            c.evaluation
                .mesh
                .effects
                .iter()
                .any(|e| e.definition.rate > 0. || e.definition.light > 0.)
        })
    }
    pub fn model_height(&self, assets: &Assets) -> Result<f32, String> {
        Ok(self.compile(0, assets)?.height)
    }
    #[cfg(feature = "graphics")]
    pub fn objects(
        &self,
        entity: &Entity,
        assets: &Assets,
        id: GeoId,
        feet: Vec3<f32>,
        height: f32,
        animation: &str,
        seconds: f32,
        vm: &mut SceneVM,
        uploaded: &mut FxHashSet<Uuid>,
        ground_flat: bool,
    ) -> Result<Vec<DynamicObject>, String> {
        self.objects_at(
            entity,
            assets,
            id,
            feet,
            height,
            animation,
            seconds,
            seconds,
            vm,
            uploaded,
            ground_flat,
        )
    }
    #[cfg(feature = "graphics")]
    pub fn objects_at(
        &self,
        entity: &Entity,
        assets: &Assets,
        id: GeoId,
        feet: Vec3<f32>,
        height: f32,
        animation: &str,
        seconds: f32,
        surface_seconds: f32,
        vm: &mut SceneVM,
        uploaded: &mut FxHashSet<Uuid>,
        ground_flat: bool,
    ) -> Result<Vec<DynamicObject>, String> {
        self.objects_with_pose(
            entity,
            assets,
            id,
            feet,
            height,
            animation,
            seconds,
            surface_seconds,
            vm,
            uploaded,
            ground_flat,
            None,
        )
    }
    #[cfg(feature = "graphics")]
    pub fn objects_with_pose(
        &self,
        entity: &Entity,
        assets: &Assets,
        id: GeoId,
        feet: Vec3<f32>,
        height: f32,
        animation: &str,
        seconds: f32,
        surface_seconds: f32,
        vm: &mut SceneVM,
        uploaded: &mut FxHashSet<Uuid>,
        ground_flat: bool,
        joints: Option<&[denrim_materials::geometry::rig::Pose]>,
    ) -> Result<Vec<DynamicObject>, String> {
        meshes::objects(
            self,
            entity,
            assets,
            id,
            feet,
            height,
            animation,
            seconds,
            surface_seconds,
            vm,
            uploaded,
            ground_flat,
            joints,
            &mut None,
        )
    }
}
fn disable_effects(doc: &mut Document) {
    for tile in doc.rows.iter_mut().flat_map(|r| &mut r.tiles) {
        match &mut tile.op {
            Operation::ParticleEffect { effect } => {
                effect.rate = 0.;
                effect.light = 0.;
            }
            Operation::Group { document, .. } | Operation::Asset { document, .. } => {
                disable_effects(document)
            }
            _ => {}
        }
    }
}
fn palette_color(palette: &ThePalette, index: u16) -> [u8; 4] {
    palette
        .colors
        .get(index as usize)
        .and_then(|c| c.as_ref())
        .map(|c| c.to_u8_array())
        .unwrap_or([128, 128, 128, 255])
}
#[cfg(test)]
fn raster_sprite(mesh: &Mesh, c: &Compiled, yaw: f32, palette: &ThePalette) -> Vec<u8> {
    raster_sprite_at(mesh, c, yaw, palette, 0., false)
}
#[cfg(test)]
fn raster_sprite_at(
    mesh: &Mesh,
    c: &Compiled,
    yaw: f32,
    palette: &ThePalette,
    seconds: f32,
    isometric: bool,
) -> Vec<u8> {
    (*raster_sprite_frame(mesh, c, yaw, palette, seconds, isometric).pixels).clone()
}
fn raster_sprite_frame(
    mesh: &Mesh,
    c: &Compiled,
    yaw: f32,
    palette: &ThePalette,
    seconds: f32,
    isometric: bool,
) -> crate::isometric::AvatarSprite {
    let size = if isometric { 128 } else { SPRITE_SIZE };
    let mut rgba = vec![0; size * size * 4];
    let mut surface = isometric.then(|| crate::isometric::surface::SurfaceFrame::new(size * size));
    let mut depth = vec![f32::NEG_INFINITY; size * size];
    let (s, co) = yaw.to_radians().sin_cos();
    let rotate = |p: [f32; 3]| [co * p[0] + s * p[2], p[1], -s * p[0] + co * p[2]];
    let project = |p: [f32; 3]| {
        let p = rotate(p);
        if isometric {
            return [
                64. + (p[0] + p[2]) * 32.,
                100. + (p[2] - p[0]) * 16. - (p[1] - c.floor) * 39.1918359,
                (p[2] - p[0]) * 0.612372436 + p[1] * 0.5,
            ];
        }
        [
            size as f32 * (0.5 + p[0] / (2. * c.extent)),
            size as f32
                * (0.5 - (p[1] - c.floor - c.height * 0.5) * 0.94 / (2. * c.extent)
                    + p[2] * 0.34 / (2. * c.extent)),
            p[2] * 0.94 + p[1] * 0.34,
        ]
    };
    let edge = |a: [f32; 3], b: [f32; 3], x: f32, y: f32| {
        (b[0] - a[0]) * (y - a[1]) - (b[1] - a[1]) * (x - a[0])
    };
    let colors = palette
        .colors
        .iter()
        .flatten()
        .map(|c| c.to_u8_array())
        .collect::<Vec<_>>();
    let mut nearest_colors = FxHashMap::default();
    for (part, material) in mesh.parts.iter().zip(&c.materials) {
        if !material.visible {
            continue;
        }
        let (_, t, packed) = material.at(seconds);
        for tri in part.vertices.chunks_exact(3) {
            let p = [
                project(tri[0].position),
                project(tri[1].position),
                project(tri[2].position),
            ];
            let area = edge(p[0], p[1], p[2][0], p[2][1]);
            if area.abs() < 1e-6 {
                continue;
            }
            let minx = p
                .iter()
                .map(|p| p[0])
                .fold(f32::INFINITY, f32::min)
                .floor()
                .max(0.) as usize;
            let maxx = p
                .iter()
                .map(|p| p[0])
                .fold(f32::NEG_INFINITY, f32::max)
                .ceil()
                .min(size as f32) as usize;
            let miny = p
                .iter()
                .map(|p| p[1])
                .fold(f32::INFINITY, f32::min)
                .floor()
                .max(0.) as usize;
            let maxy = p
                .iter()
                .map(|p| p[1])
                .fold(f32::NEG_INFINITY, f32::max)
                .ceil()
                .min(size as f32) as usize;
            let n = rotate(tri[0].normal);
            let light =
                (0.65 + (n[0] * -0.3 + n[1] * 0.7 + n[2] * 0.5).max(0.) * 0.35).clamp(0., 1.);
            for y in miny..maxy {
                for x in minx..maxx {
                    let weights = [
                        edge(p[1], p[2], x as f32 + 0.5, y as f32 + 0.5) / area,
                        edge(p[2], p[0], x as f32 + 0.5, y as f32 + 0.5) / area,
                    ];
                    let w = [weights[0], weights[1], 1. - weights[0] - weights[1]];
                    if w.iter().any(|v| *v < -1e-5) {
                        continue;
                    }
                    let z = (0..3).map(|i| p[i][2] * w[i]).sum::<f32>();
                    let at = y * size + x;
                    if z <= depth[at] {
                        continue;
                    }
                    let uv: [f32; 2] =
                        std::array::from_fn(|k| (0..3).map(|i| tri[i].uv[k] * w[i]).sum());
                    let tx = ((uv[0].rem_euclid(1.) * t.width as f32) as usize).min(t.width - 1);
                    let ty = ((uv[1].rem_euclid(1.) * t.height as f32) as usize).min(t.height - 1);
                    let off = (ty * t.width + tx) * 4;
                    if t.data[off + 3] < 128 {
                        continue;
                    }
                    let roughness = (packed[off] & 15) as f32 / 15.;
                    let metallic = (packed[off] >> 4) as f32 / 15.;
                    let emission = (packed[off + 1] >> 4) as f32 / 15.;
                    if let Some(surface) = &mut surface {
                        let normal = rotate(std::array::from_fn(|k| {
                            (0..3).map(|i| tri[i].normal[k] * w[i]).sum()
                        }));
                        let height =
                            (0..3).map(|i| tri[i].position[1] * w[i]).sum::<f32>() - c.floor;
                        surface.set(
                            at,
                            Vec3::from(normal),
                            height,
                            roughness,
                            metallic,
                            emission,
                        );
                        rgba[at * 4..at * 4 + 4].copy_from_slice(&[
                            t.data[off],
                            t.data[off + 1],
                            t.data[off + 2],
                            255,
                        ]);
                        depth[at] = z;
                        continue;
                    }
                    let specular = n[2].max(0.).powf(4. + roughness * 28.)
                        * (1. - roughness)
                        * (0.06 + metallic * 0.3);
                    let illumination = light * (1. - emission) + emission;
                    let rgb: [u8; 3] = std::array::from_fn(|k| {
                        (t.data[off + k] as f32 * illumination
                            + specular
                                * (255. * (1. - metallic) + t.data[off + k] as f32 * metallic))
                            .clamp(0., 255.) as u8
                    });
                    let nearest = nearest_colors.entry(rgb).or_insert_with(|| {
                        colors
                            .iter()
                            .min_by_key(|c| {
                                (0..3)
                                    .map(|k| (c[k] as i32 - rgb[k] as i32).pow(2))
                                    .sum::<i32>()
                            })
                            .copied()
                            .unwrap_or([rgb[0], rgb[1], rgb[2], 255])
                    });
                    rgba[at * 4..at * 4 + 4]
                        .copy_from_slice(&[nearest[0], nearest[1], nearest[2], 255]);
                    depth[at] = z;
                }
            }
        }
    }
    crate::isometric::AvatarSprite {
        pixels: Arc::new(rgba),
        surface: surface.map(Arc::new),
    }
}

struct Transition {
    token: (String, i32, u64),
    from: Option<Vec<denrim_materials::geometry::rig::Pose>>,
    last: Vec<denrim_materials::geometry::rig::Pose>,
    started: f32,
}
/// Presentation markers sampled once per entity, even when both mesh/effect
/// render passes request the same frame. Gameplay remains server authoritative.
#[derive(Clone, Debug, PartialEq)]
pub struct AnimationEvent {
    pub entity: GeoId,
    pub animation: String,
    pub name: String,
}
pub(crate) struct AnimationSample {
    pub animation: String,
    pub time: f32,
    pub joints: Vec<denrim_materials::geometry::rig::Pose>,
}
#[derive(Default)]
pub struct Runtime {
    frame: usize,
    frame_at: Option<instant::Instant>,
    pub time: f32,
    playback: FxHashMap<GeoId, (String, i32, f32)>,
    transitions: FxHashMap<GeoId, Transition>,
    walk_clocks: FxHashMap<GeoId, (f32, f32)>,
    event_cursors: FxHashMap<GeoId, ((String, i32, u64), f32)>,
    events: Vec<AnimationEvent>,
    uploaded: FxHashSet<Uuid>,
    #[cfg(feature = "graphics")]
    meshes: FxHashMap<GeoId, Option<meshes::CachedObjects>>,
}
impl Runtime {
    fn worn_model<'a>(source: &str, assets: &'a Assets) -> Option<&'a Arc<Appearance>> {
        assets
            .procedural_avatars
            .get(source.strip_prefix("ruleset:geometry/").unwrap_or(source))
    }
    pub fn resolve_wearable(item: &crate::Item, assets: &Assets) -> Option<Wearable> {
        let creator = item.creator_id.to_string();
        let keys = [
            Some(creator.as_str()),
            item.attributes.get_str("creator_template_id"),
            item.attributes.get_str("ruleset_path"),
            item.attributes.get_str("ruleset_id"),
            Some(item.item_type.as_str()),
        ];
        for key in keys.into_iter().flatten() {
            if let Some(worn) = assets.item_wearables.get(&key.trim().to_ascii_lowercase()) {
                return worn.clone();
            }
        }
        Wearable::from_item(item).ok().flatten()
    }
    pub fn resolve_item<'a>(item: &crate::Item, assets: &'a Assets) -> Option<&'a Arc<Appearance>> {
        if !item.attributes.get_bool_default("active", false)
            && let Some(name) = item.attributes.get_str("appearance_geometry_off")
        {
            return assets
                .procedural_avatars
                .get(name.strip_prefix("ruleset:geometry/").unwrap_or(name));
        }
        let template = item.attributes.get_str("creator_template_id");
        let creator = item.creator_id.to_string();
        let keys = [
            Some(creator.as_str()),
            template,
            item.attributes.get_str("ruleset_path"),
            item.attributes.get_str("ruleset_id"),
            Some(item.item_type.as_str()),
        ];
        let name = keys
            .into_iter()
            .flatten()
            .find_map(|k| {
                assets
                    .item_models
                    .get(&k.trim().to_ascii_lowercase())
                    .map(String::as_str)
            })
            .or_else(|| item.attributes.get_str("appearance_geometry"))?;
        let model = assets
            .procedural_avatars
            .get(name.strip_prefix("ruleset:geometry/").unwrap_or(name))?;
        Some(if item.attributes.get_bool_default("active", false) {
            model.state_on.as_ref().unwrap_or(model)
        } else { model })
    }
    pub fn item_entity(item: &crate::Item) -> Entity {
        let degrees = item.attributes.get_float_default("appearance_yaw", 0.);
        let yaw = degrees.to_radians();
        let mut entity = Entity { orientation: vek::Vec2::new(yaw.sin(), yaw.cos()), ..Default::default() };
        entity.set_attribute("appearance_yaw", crate::Value::Float(degrees));
        entity
    }
    pub fn isometric_item_sprite(&mut self, item: &crate::Item, assets: &Assets) -> Option<crate::isometric::AvatarSprite> {
        let model = Self::resolve_item(item, assets)?;
        model.sprite_camera(&Self::item_entity(item), assets, "idle", 0., self.time, None, true).ok()
    }
    /// A separately authored Off model owns its effects too (for example smoke).
    pub fn item_effects_enabled(item: &crate::Item) -> bool {
        !matches!(
            item.attributes.get("active"),
            Some(crate::Value::Bool(false))
        ) || item
            .attributes
            .get_str("appearance_geometry_off")
            .is_some_and(|s| !s.trim().is_empty())
    }
    pub fn item_uses_effects(item: &crate::Item, assets: &Assets) -> bool {
        Self::resolve_item(item, assets).is_some_and(|a| a.has_effects)
            || item
                .attributes
                .get_str("appearance_geometry")
                .and_then(|name| {
                    assets
                        .procedural_avatars
                        .get(name.strip_prefix("ruleset:geometry/").unwrap_or(name))
                })
                .is_some_and(|a| a.has_effects)
    }
    #[cfg(feature = "graphics")]
    pub fn item_sprite(
        &mut self,
        vm: &mut SceneVM,
        item: &crate::Item,
        assets: &Assets,
        id: GeoId,
    ) -> bool {
        if !Self::item_uses_effects(item, assets) {
            return false;
        }
        let Some(model) = Self::resolve_item(item, assets) else {
            return false;
        };
        match model.sprite_at(&Entity::default(), assets, "idle", 0., self.time) {
            Ok(rgba) => {
                vm.execute(Atom::SetAvatarBillboardData {
                    id,
                    size: SPRITE_SIZE as u32,
                    rgba: (*rgba).clone(),
                });
                true
            }
            Err(_) => false,
        }
    }
    #[cfg(feature = "graphics")]
    pub fn item_objects(
        &mut self,
        vm: &mut SceneVM,
        item: &crate::Item,
        assets: &Assets,
        id: GeoId,
        feet: Vec3<f32>,
    ) -> bool {
        let Some(model) = Self::resolve_item(item, assets) else {
            return false;
        };
        let entity = Self::item_entity(item);
        let height = match model.compile(0, assets) {
            Ok(c) => c.height,
            Err(_) => return false,
        };
        match meshes::objects(
            model,
            &entity,
            assets,
            id,
            feet,
            height * item.attributes.get_float_default("appearance_scale", 1.),
            "idle",
            0.,
            self.time,
            vm,
            &mut self.uploaded,
            item.attributes
                .get_bool_default("appearance_ground_flat", false),
            None,
            self.meshes.entry(id).or_default(),
        ) {
            Ok(objects) => {
                for object in objects {
                    vm.execute(Atom::AddDynamic { object });
                }
                true
            }
            Err(e) => {
                eprintln!("Item geometry: {e}");
                false
            }
        }
    }
    pub fn tick(&mut self, frame: usize, tick_seconds: f32) {
        let now = instant::Instant::now();
        if frame < self.frame {
            self.playback.clear();
            self.transitions.clear();
            self.walk_clocks.clear();
            self.event_cursors.clear();
            self.events.clear();
        }
        if self.frame != frame || self.frame_at.is_none() {
            self.frame = frame;
            self.frame_at = Some(now);
        }
        self.time = frame as f32 * tick_seconds
            + self
                .frame_at
                .map_or(0., |at| (now - at).as_secs_f32().min(tick_seconds));
    }
    pub(crate) fn phase(&mut self, id: GeoId, entity: &Entity) -> (String, f32) {
        let animation = entity
            .attributes
            .get_str("avatar_animation")
            .unwrap_or("idle")
            .to_ascii_lowercase();
        let serial = entity.attributes.get_int_default("avatar_attack_serial", 0);
        let state = self
            .playback
            .entry(id)
            .or_insert((animation.clone(), serial, self.time));
        if state.0 != animation || (animation == "attack" && state.1 != serial) {
            *state = (animation.clone(), serial, self.time);
        }
        (animation, (self.time - state.2).max(0.))
    }
    pub(crate) fn sample(
        &mut self,
        id: GeoId,
        entity: &Entity,
        model: &Appearance,
        assets: &Assets,
        two_d: bool,
    ) -> Result<AnimationSample, String> {
        let (animation, mut time) = self.phase(id, entity);
        let (c, variant) = model.compiled_for(entity, assets)?;
        if animation == "walk" {
            let duration = Appearance::motion(&c, &animation).map_or(1., |m| m.duration);
            let stride = entity
                .attributes
                .get_float_default("avatar_stride", 0.9)
                .max(0.05);
            let speed = entity.attributes.get_float_default(
                "avatar_walk_rate",
                entity
                    .attributes
                    .get_float_default("avatar_move_speed", stride / duration)
                    * duration
                    / stride,
            );
            let speed = if speed.is_finite() {
                speed.clamp(0., 4.)
            } else {
                1.
            };
            let clock = self.walk_clocks.entry(id).or_insert((self.time, 0.));
            clock.1 += (self.time - clock.0).max(0.) * speed;
            clock.0 = self.time;
            time = clock.1;
        } else {
            self.walk_clocks.remove(&id);
        }
        let Some(rig) = c.evaluation.mesh.rig.as_ref() else {
            return Ok(AnimationSample {
                animation,
                time,
                joints: vec![],
            });
        };
        let motion = Appearance::motion(&c, &animation);
        let mut pose_seconds = Appearance::animation_seconds(&c, entity, &animation, time);
        if two_d {
            // Quantize *after* wrapping the clip. Quantizing the ever-growing
            // clock first produces different fractional poses on every loop
            // when duration is not an exact multiple of the sprite interval.
            // Those joint bits defeated the sprite cache and eventually cleared it.
            if let Some(motion) = motion {
                pose_seconds = if motion.looping {
                    pose_seconds.rem_euclid(motion.duration)
                } else {
                    pose_seconds.min(motion.duration)
                };
            }
            let complete =
                motion.is_some_and(|m| !m.looping && pose_seconds + 0.000001 >= m.duration);
            if !complete {
                pose_seconds = (pose_seconds * FPS).floor() / FPS;
            }
        }
        let pose = rig
            .sample(motion, pose_seconds)
            .map_err(|e| e.to_string())?;
        let token = (
            animation.clone(),
            entity.attributes.get_int_default("avatar_attack_serial", 0),
            fingerprint(&(variant, model.signature)),
        );
        if let Some(motion) = Appearance::motion(&c, &animation) {
            let sample = Appearance::animation_seconds(&c, entity, &animation, time);
            let cursor = self
                .event_cursors
                .entry(id)
                .or_insert((token.clone(), -f32::EPSILON));
            if cursor.0 != token {
                *cursor = (token.clone(), -f32::EPSILON);
            }
            if sample >= cursor.1 {
                for event in motion.events_between(cursor.1, sample) {
                    if self.events.len() >= 1024 {
                        self.events.drain(..512);
                    }
                    self.events.push(AnimationEvent {
                        entity: id,
                        animation: animation.clone(),
                        name: event.name.clone(),
                    });
                }
                cursor.1 = sample;
            }
        }
        let now = if two_d {
            (self.time * FPS).floor() / FPS
        } else {
            self.time
        };
        let state = self.transitions.entry(id).or_insert_with(|| Transition {
            token: token.clone(),
            from: None,
            last: pose.clone(),
            started: now,
        });
        if state.token != token {
            state.from = (state.token.2 == token.2 && state.last.len() == pose.len())
                .then(|| state.last.clone());
            state.token = token;
            state.started = now;
        }
        let duration = entity
            .attributes
            .get_float_default("avatar_blend_time", 0.12);
        let duration = if duration.is_finite() {
            duration.clamp(0., 1.)
        } else {
            0.12
        };
        let weight = if duration <= 0. {
            1.
        } else {
            ((now - state.started) / duration).clamp(0., 1.)
        };
        let joints = if let Some(from) = state.from.as_ref().filter(|_| weight < 1.) {
            rig.blend(from, &pose, weight).map_err(|e| e.to_string())?
        } else {
            state.from = None;
            pose
        };
        state.last = joints.clone();
        Ok(AnimationSample {
            animation,
            time,
            joints,
        })
    }
    pub fn resolve<'a>(entity: &Entity, assets: &'a Assets) -> Option<&'a Arc<Appearance>> {
        let name = entity
            .attributes
            .get_id("avatar_id")
            .map(|id| id.to_string())
            .or_else(|| entity.attributes.get_str("avatar").map(str::to_owned))
            .or_else(|| {
                if entity.attributes.get("source").is_none()
                    && entity.attributes.get("tile_id").is_none()
                {
                    assets.default_avatar.clone()
                } else {
                    None
                }
            })?;
        assets.procedural_avatars.get(&name).or_else(|| {
            assets
                .procedural_avatars
                .iter()
                .find(|(key, _)| key.eq_ignore_ascii_case(&name))
                .map(|(_, a)| a)
        })
    }
    pub fn isometric_sprite(
        &mut self,
        entity: &Entity,
        assets: &Assets,
        id: GeoId,
    ) -> Option<crate::isometric::AvatarSprite> {
        let asset = Self::resolve(entity, assets)?;
        let sample = self.sample(id, entity, asset, assets, true).ok()?;
        asset
            .sprite_camera(
                entity,
                assets,
                &sample.animation,
                sample.time,
                self.time,
                (!sample.joints.is_empty()).then_some(sample.joints.as_slice()),
                true,
            )
            .ok()
    }
    #[cfg(feature = "graphics")]
    pub fn sprite(
        &mut self,
        vm: &mut SceneVM,
        entity: &Entity,
        assets: &Assets,
        id: GeoId,
        style: Option<crate::avatar_builder::AvatarFrameStyle>,
    ) -> bool {
        let Some(asset) = Self::resolve(entity, assets) else {
            return false;
        };
        let sample = match self.sample(id, entity, asset, assets, true) {
            Ok(s) => s,
            Err(_) => return false,
        };
        match asset.sprite_with_pose(
            entity,
            assets,
            &sample.animation,
            sample.time,
            self.time,
            (!sample.joints.is_empty()).then_some(sample.joints.as_slice()),
        ) {
            Ok(pixels) => {
                let mut rgba = (*pixels).clone();
                if let Some(style) = style {
                    crate::avatar_builder::AvatarRuntimeBuilder::draw_alpha_outline(
                        SPRITE_SIZE,
                        &mut rgba,
                        style.outline_color,
                        style.outline_thickness,
                    );
                }
                vm.execute(Atom::SetAvatarBillboardData {
                    id,
                    size: SPRITE_SIZE as u32,
                    rgba,
                });
                true
            }
            Err(e) => {
                eprintln!("Procedural avatar: {e}");
                false
            }
        }
    }
    #[cfg(feature = "graphics")]
    pub fn objects(
        &mut self,
        vm: &mut SceneVM,
        entity: &Entity,
        assets: &Assets,
        id: GeoId,
        feet: Vec3<f32>,
        height: f32,
    ) -> bool {
        let Some(asset) = Self::resolve(entity, assets) else {
            return false;
        };
        let sample = match self.sample(id, entity, asset, assets, false) {
            Ok(s) => s,
            Err(_) => return false,
        };
        match meshes::objects(
            asset,
            entity,
            assets,
            id,
            feet,
            height,
            &sample.animation,
            sample.time,
            self.time,
            vm,
            &mut self.uploaded,
            false,
            (!sample.joints.is_empty()).then_some(sample.joints.as_slice()),
            self.meshes.entry(id).or_default(),
        ) {
            Ok(objects) => {
                for object in objects {
                    vm.execute(Atom::AddDynamic { object });
                }
                true
            }
            Err(e) => {
                eprintln!("Procedural avatar: {e}");
                false
            }
        }
    }
    pub fn drain_animation_events(&mut self) -> impl Iterator<Item = AnimationEvent> + '_ {
        self.events.drain(..)
    }
    pub fn retain(&mut self, ids: &FxHashSet<GeoId>) {
        #[cfg(feature = "graphics")]
        self.meshes.retain(|id, _| ids.contains(id));
        self.playback.retain(|id, _| ids.contains(id));
        self.transitions.retain(|id, _| ids.contains(id));
        self.walk_clocks.retain(|id, _| ids.contains(id));
        self.event_cursors.retain(|id, _| ids.contains(id));
        self.events.retain(|event| ids.contains(&event.entity));
    }
    pub fn reset_uploads(&mut self) {
        self.uploaded.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Item, Value};
    #[test]
    fn isometric_mz_character_uses_directional_walk_poses_without_outlines() {
        let (assets, model, mut entity) = fixture();
        entity.orientation = Vec2::new(0., 1.);
        let first = model
            .sprite_isometric(&entity, &assets, "walk", 0., 0., None)
            .unwrap();
        let next = model
            .sprite_isometric(&entity, &assets, "walk", 0.25, 0., None)
            .unwrap();
        assert_eq!(first.len(), 128 * 128 * 4);
        assert_ne!(first, next);
        assert!(first.chunks_exact(4).any(|c| c[3] == 255));
        assert!(first.chunks_exact(4).all(|c| c[3] == 0 || c[3] == 255));
        entity.orientation = Vec2::new(1., 0.);
        let right = model
            .sprite_isometric(&entity, &assets, "walk", 0., 0., None)
            .unwrap();
        assert_ne!(first, right);
        assert_eq!(
            model.sprite(&entity, &assets, "walk", 0.).unwrap().len(),
            SPRITE_SIZE * SPRITE_SIZE * 4
        );
    }
    #[test]
    fn looping_2d_pose_reuses_the_same_sprite_across_non_aligned_clip_cycles() {
        let (assets, original, mut entity) = fixture();
        let mut doc = original.document.clone();
        for tile in doc.rows.iter_mut().flat_map(|r| &mut r.tiles) {
            if let Operation::Motion { duration, .. } = &mut tile.op {
                *duration = 1.3;
            }
        }
        let model = Appearance::new(&doc.save().unwrap(), &[], 991).unwrap();
        entity.set_attribute("avatar_animation", Value::Str("idle".into()));
        let mut runtime = Runtime::default();
        let id = GeoId::Unknown(991);
        runtime.sample(id, &entity, &model, &assets, true).unwrap();
        let mut reference = None;
        for cycle in 0..80 {
            runtime.time = cycle as f32 * 1.3 + 0.17;
            let sample = runtime.sample(id, &entity, &model, &assets, true).unwrap();
            let pixels = model
                .sprite_with_pose(
                    &entity,
                    &assets,
                    &sample.animation,
                    sample.time,
                    0.,
                    Some(&sample.joints),
                )
                .unwrap();
            if let Some(reference) = &reference {
                assert!(
                    Arc::ptr_eq(reference, &pixels),
                    "cycle {cycle} missed the retained sprite"
                );
            } else {
                reference = Some(pixels);
            }
        }
        assert_eq!(model.sprites.lock().unwrap().len(), 1);
    }

    #[test]
    fn integration_pass_mesh_textures_animate_without_recompiling_geometry() {
        let mut assets = Assets::default();
        assets.palette = eldiron_ruleset::palette_template_from_source(
            eldiron_ruleset::latest_official_ruleset(),
        )
        .unwrap();
        let mut doc = Document::load(denrim_materials::geometry::SWORD_EXAMPLE).unwrap();
        doc.rows
            .iter_mut()
            .find(|r| r.id == doc.output)
            .unwrap()
            .tiles
            .push(denrim_materials::geometry::Tile {
                id: "animated-surface".into(),
                op: Operation::Texture {
                    preserve_uv: true,
                    parts: vec![],
                    source: "test-animation".into(),
                    bundle: None,
                    size: 16,
                    axes: [0, 1],
                    origin: [0.; 2],
                    span: [1.; 2],
                },
            });
        let a = Texture::new(
            assets.palette.colors[8]
                .as_ref()
                .unwrap()
                .to_u8_array()
                .repeat(4),
            2,
            2,
        );
        let b = Texture::new(
            assets.palette.colors[20]
                .as_ref()
                .unwrap()
                .to_u8_array()
                .repeat(4),
            2,
            2,
        );
        assets
            .ruleset_materials
            .insert("test-animation".into(), a.clone());
        assets
            .material_animations
            .insert("test-animation".into(), (vec![a, b], 1.));
        let model = Appearance::new(&doc.save().unwrap(), &[], 1).unwrap();
        let entity = Entity::default();
        let first = model.sprite_at(&entity, &assets, "idle", 0., 0.1).unwrap();
        let later = model.sprite_at(&entity, &assets, "idle", 0., 0.6).unwrap();
        assert_ne!(first, later, "A static pose must still animate its texture");
        assert_eq!(
            first,
            model.sprite_at(&entity, &assets, "idle", 0., 1.1).unwrap()
        );
        let c = model.compiled_for(&entity, &assets).unwrap().0;
        assert_eq!(model.compiled.lock().unwrap().len(), 1);
        assert_ne!(
            c.materials[0].at(0.1).0,
            c.materials[0].at(0.6).0,
            "3D uses distinct, reusable frame IDs"
        );
    }
    #[test]
    fn integration_pass_transitions_interrupt_cleanly_and_events_fire_once() {
        let mut assets = Assets::default();
        let mut doc = Document::load(
            eldiron_ruleset::bundled_appearance_assets_for_ruleset(
                eldiron_ruleset::OFFICIAL_RULESET_ID,
                eldiron_ruleset::OFFICIAL_RULESET_VERSION,
            )
            .into_iter()
            .find(|d| d.id == "humanoid")
            .unwrap()
            .source,
        )
        .unwrap();
        for tile in doc.rows.iter_mut().flat_map(|r| &mut r.tiles) {
            if let Operation::Motion { events, .. } = &mut tile.op {
                *events = vec![denrim_materials::geometry::rig::Event {
                    time: 0.,
                    name: "start".into(),
                }];
            }
        }
        let model = Appearance::new(&doc.save().unwrap(), &[], 1).unwrap();
        assets.palette = ThePalette::default();
        let id = GeoId::Character(1);
        let mut runtime = Runtime::default();
        let mut entity = Entity::default();
        let idle = runtime.sample(id, &entity, &model, &assets, false).unwrap();
        assert_eq!(runtime.drain_animation_events().count(), 1);
        runtime.sample(id, &entity, &model, &assets, false).unwrap();
        assert_eq!(runtime.drain_animation_events().count(), 0);
        runtime.time = 0.2;
        entity.set_attribute("avatar_animation", Value::Str("walk".into()));
        let start = runtime.sample(id, &entity, &model, &assets, false).unwrap();
        assert_eq!(start.joints, idle.joints);
        runtime.time = 0.25;
        let halfway = runtime.sample(id, &entity, &model, &assets, false).unwrap();
        entity.set_attribute("avatar_animation", Value::Str("attack".into()));
        let interrupted = runtime.sample(id, &entity, &model, &assets, false).unwrap();
        assert_eq!(
            interrupted.joints, halfway.joints,
            "Interruptions must start at the displayed blended pose"
        );
        assert_eq!(runtime.drain_animation_events().count(), 2);
        runtime.time = 0.5;
        let attack = runtime.sample(id, &entity, &model, &assets, false).unwrap();
        assert_ne!(attack.joints, idle.joints);
    }
    #[test]
    fn torch_effect_follows_the_posed_off_hand_and_its_item_state() {
        let mut assets = Assets::default();
        assets.rules = eldiron_ruleset::latest_official_ruleset().into();
        assets.palette = eldiron_ruleset::palette_template_from_source(&assets.rules).unwrap();
        let sources = eldiron_ruleset::bundled_appearance_assets_for_ruleset(
            eldiron_ruleset::OFFICIAL_RULESET_ID,
            eldiron_ruleset::OFFICIAL_RULESET_VERSION,
        );
        let body = sources
            .iter()
            .find(|s| s.id == "humanoid")
            .unwrap_or(&sources[0]);
        let actor = Appearance::new(body.source, body.equipment, 7).unwrap();
        for name in ["torch", "torch_off"] {
            let source = sources.iter().find(|s| s.id == name).unwrap();
            assets.procedural_avatars.insert(
                name.into(),
                Arc::new(
                    Appearance::new(source.source, source.equipment, fingerprint(&source.source))
                        .unwrap(),
                ),
            );
        }
        let mut torch = crate::Item::default();
        torch.set_attribute("appearance_geometry", crate::Value::Str("torch".into()));
        torch.set_attribute(
            "appearance_geometry_off",
            crate::Value::Str("torch_off".into()),
        );
        torch.set_attribute("active", crate::Value::Bool(true));
        torch.set_attribute("appearance_idle", crate::Value::Str("torch-idle".into()));
        torch.set_attribute("appearance_walk", crate::Value::Str("torch-walk".into()));
        let mut actor_entity = Entity::default();
        actor_entity.equipped.insert("tool".into(), torch);
        assert!(
            actor.has_entity_effects(&actor_entity, &assets),
            "An attached torch must keep the effects path active"
        );
        let feet = Vec3::new(2., 0., 3.);
        let first = actor
            .effects(&actor_entity, &assets, feet, 2., "walk", 0., false, false)
            .unwrap();
        assert_eq!(first.len(), 1);
        let moved = actor
            .effects(&actor_entity, &assets, feet, 2., "walk", 0.25, false, false)
            .unwrap();
        assert_ne!(
            first[0].position, moved[0].position,
            "the emitter must move with the posed hand"
        );
        let offset = Vec3::new(4., 0., 2.);
        let translated = actor
            .effects(
                &actor_entity,
                &assets,
                feet + offset,
                2.,
                "walk",
                0.,
                false,
                false,
            )
            .unwrap();
        assert!((translated[0].position - first[0].position - offset).magnitude() < 0.00001);
        let screen = actor
            .effects(&actor_entity, &assets, feet, 1., "walk", 0., false, true)
            .unwrap();
        assert_eq!(screen.len(), 1);
        assert_eq!(screen[0].position.y, feet.y);
        let policy =
            eldiron_ruleset::resolve_equipment_policy(&assets.rules_table().unwrap()).unwrap();
        assert!(
            policy
                .occupied_slots("tool", Some("utility"), "tool")
                .contains("off_hand")
        );
        actor_entity
            .equipped
            .get_mut("tool")
            .unwrap()
            .set_attribute("active", crate::Value::Bool(false));
        assert!(
            actor
                .effects(&actor_entity, &assets, feet, 2., "walk", 0., false, false)
                .unwrap()
                .is_empty()
        );
        // Off models remain independently authored, including optional smoke effects.
        actor_entity
            .equipped
            .get_mut("tool")
            .unwrap()
            .set_attribute("appearance_geometry_off", crate::Value::Str("torch".into()));
        assert_eq!(
            actor
                .effects(&actor_entity, &assets, feet, 2., "idle", 0., false, false)
                .unwrap()
                .len(),
            1
        );
        actor_entity.equipped.clear();
        assert!(
            !actor.has_entity_effects(&actor_entity, &assets),
            "A plain body must skip effect posing"
        );
        assert!(
            actor
                .effects(&actor_entity, &assets, feet, 2., "idle", 0., false, false)
                .unwrap()
                .is_empty()
        );
    }

    pub(super) fn fixture() -> (Assets, Appearance, Entity) {
        let mut assets = Assets::default();
        assets.palette = eldiron_ruleset::palette_template_from_source(
            eldiron_ruleset::latest_official_ruleset(),
        )
        .unwrap();
        let asset = eldiron_ruleset::bundled_appearance_assets_for_ruleset(
            eldiron_ruleset::OFFICIAL_RULESET_ID,
            eldiron_ruleset::OFFICIAL_RULESET_VERSION,
        )
        .remove(0);
        for def in eldiron_ruleset::bundled_material_assets_for_ruleset(
            eldiron_ruleset::OFFICIAL_RULESET_ID,
            eldiron_ruleset::OFFICIAL_RULESET_VERSION,
        ) {
            let editable = crate::ruleset_material::editable(&def, &assets.palette).unwrap();
            let texture =
                crate::ruleset_material::bake(&editable, &assets.palette, def.resolution).unwrap();
            assets
                .ruleset_materials
                .insert(format!("ruleset:material/{}", def.id), texture);
        }
        let appearance = Appearance::new(asset.source, asset.equipment, 7).unwrap();
        for source in eldiron_ruleset::bundled_appearance_assets_for_ruleset(
            eldiron_ruleset::OFFICIAL_RULESET_ID,
            eldiron_ruleset::OFFICIAL_RULESET_VERSION,
        )
        .into_iter()
        .skip(1)
        {
            let model = Arc::new(
                Appearance::new(source.source, source.equipment, fingerprint(&source.source))
                    .unwrap_or_else(|e| panic!("{}: {e}", source.id)),
            );
            assets
                .procedural_avatars
                .insert(source.id.into(), model.clone());
            assets
                .procedural_avatars
                .insert(source.asset_id.into(), model);
        }
        let mut entity = Entity::default();
        entity.set_attribute("avatar", Value::Str("humanoid".into()));
        entity.orientation = Vec2::new(0., 1.);
        (assets, appearance, entity)
    }
    #[test]
    fn bow_and_spear_keep_two_hand_contact_and_release_the_arrow_on_every_race() {
        use denrim_materials::geometry::rig::Pose;
        let (mut assets, human, mut entity) = fixture();
        assets
            .procedural_avatars
            .insert("humanoid".into(), Arc::new(human));
        let templates = eldiron_ruleset::ruleset_item_templates_from_source(
            eldiron_ruleset::latest_official_ruleset(),
        )
        .unwrap();
        let make_item = |name: &str| {
            let mut item = Item::default();
            crate::server::data::apply_item_data(
                &mut item,
                &templates.iter().find(|t| t.id == name).unwrap().data,
            );
            item
        };
        let distance =
            |a: [f32; 3], b: [f32; 3]| (0..3).map(|k| (a[k] - b[k]).powi(2)).sum::<f32>().sqrt();
        for race in ["humanoid", "orc", "skeleton"] {
            let actor = &assets.procedural_avatars[race];
            for weapon in ["hunting_bow", "training_spear"] {
                entity.equipped.clear();
                for (slot, name) in [
                    ("main_hand", weapon),
                    ("off_hand", "round_shield"),
                    ("torso", "linen_shirt"),
                    ("legs", "wool_trousers"),
                    ("feet", "leather_shoes"),
                ] {
                    entity.equipped.insert(slot.into(), make_item(name));
                }
                let (c, _) = actor.compiled_for(&entity, &assets).unwrap();
                let rig = c.evaluation.mesh.rig.as_ref().unwrap();
                assert!(
                    !c.evaluation
                        .mesh
                        .attachments
                        .iter()
                        .any(|a| a.id == "shield")
                );
                let motion = Appearance::motion(&c, "attack").unwrap();
                assert_eq!(motion.duration, attack_duration(&entity));
                let mut tips = vec![];
                for phase in [0., 0.12, 0.3, 0.45, 0.55, 0.6, 0.64, 0.75, 0.86, 1.] {
                    let posed = c
                        .evaluation
                        .mesh
                        .posed(Some(motion), phase * motion.duration)
                        .unwrap();
                    let right =
                        posed.joints[rig.index("right-hand").unwrap()].point([0., -0.045, 0.018]);
                    let left =
                        posed.joints[rig.index("left-hand").unwrap()].point([0., -0.045, 0.018]);
                    if weapon == "hunting_bow" {
                        let nock = posed.joints[rig.index("bow-upper-nock").unwrap()].translation;
                        if phase <= 0.6 {
                            assert!(
                                distance(right, nock) < 0.018,
                                "{race}/{phase}: bow string detached by {}",
                                distance(right, nock)
                            );
                        }
                        let other = posed.joints[rig.index("bow-lower-nock").unwrap()].translation;
                        assert!(distance(nock, other) < 0.001);
                        let root = posed.joints[rig.index("bow-root").unwrap()].translation;
                        assert!(distance(left, root) < 0.001);
                        let arrow = posed
                            .mesh
                            .parts
                            .iter()
                            .find(|p| p.id == "bow-arrow-shaft")
                            .unwrap();
                        assert_eq!(arrow.vertices.is_empty(), phase >= 0.84 && phase < 0.98);
                        tips.push(posed.joints[rig.index("bow-arrow").unwrap()].translation);
                    } else {
                        let attachment = c
                            .evaluation
                            .mesh
                            .attachments
                            .iter()
                            .find(|a| a.id == "sword")
                            .unwrap();
                        let transform = posed.joints[attachment.joint]
                            .compose(Pose::from_frame(&attachment.socket))
                            .compose(Pose::from_frame(&attachment.grip).inverse());
                        assert!(distance(right, transform.point([0.; 3])) < 0.001);
                        assert!(
                            distance(left, transform.point([0., 0.24, 0.])) < 0.018,
                            "{race}/{phase}: spear support hand detached by {}",
                            distance(left, transform.point([0., 0.24, 0.]))
                        );
                        tips.push(transform.point([0., 1.05, 0.]));
                    }
                    assert!(
                        posed.mesh.parts.iter().flat_map(|p| &p.vertices).all(|v| v
                            .position
                            .iter()
                            .chain(&v.normal)
                            .all(|n| n.is_finite()))
                    );
                }
                assert!(
                    tips[7][2] - tips[4][2] > 0.6 || weapon == "training_spear",
                    "{race}: arrow did not fly forwards"
                );
                if weapon == "training_spear" {
                    assert!(
                        tips[4][2] - tips[0][2] > 0.22,
                        "{race}: spear did not thrust forwards"
                    );
                }
                for heading in 0..8 {
                    let a = (heading as f32 * 45.).to_radians();
                    entity.orientation = Vec2::new(a.sin(), a.cos());
                    let start = actor.sprite(&entity, &assets, "attack", 0.).unwrap();
                    let draw = actor
                        .sprite(&entity, &assets, "attack", attack_duration(&entity) * 0.55)
                        .unwrap();
                    assert_ne!(start, draw, "{race}/{weapon}/{heading}");
                    let end = actor
                        .sprite(&entity, &assets, "attack", attack_duration(&entity))
                        .unwrap();
                    let final_mesh = c
                        .evaluation
                        .mesh
                        .posed(Some(motion), motion.duration)
                        .unwrap()
                        .mesh;
                    assert_eq!(
                        *end,
                        raster_sprite(&final_mesh, &c, heading as f32 * 45., &assets.palette),
                        "{race}/{weapon}: final sprite key was skipped"
                    );
                }
            }
        }
    }
    #[test]
    fn worn_accessories_follow_each_race_through_clothing_and_motion() {
        let (mut assets, human, mut entity) = fixture();
        assets
            .procedural_avatars
            .insert("humanoid".into(), Arc::new(human));
        let templates = eldiron_ruleset::ruleset_item_templates_from_source(
            eldiron_ruleset::latest_official_ruleset(),
        )
        .unwrap();
        let make_item = |name: &str| {
            let mut item = Item::default();
            crate::server::data::apply_item_data(
                &mut item,
                &templates.iter().find(|t| t.id == name).unwrap().data,
            );
            item
        };
        for race in ["humanoid", "orc", "skeleton"] {
            let actor = &assets.procedural_avatars[race];
            for (focus, joint) in [("ritual_censer", "pelvis"), ("sunward_charm", "chest")] {
                entity.equipped.clear();
                for (slot, name) in [
                    ("torso", "chain_shirt"),
                    ("legs", "wool_trousers"),
                    ("container", "small_bag"),
                    ("focus", focus),
                    ("main_hand", "training_sword"),
                    ("off_hand", "round_shield"),
                ] {
                    entity.equipped.insert(slot.into(), make_item(name));
                }
                let (compiled, _) = actor.compiled_for(&entity, &assets).unwrap();
                let rig = compiled.evaluation.mesh.rig.as_ref().unwrap();
                for (name, joint) in [("small_bag", "pelvis"), (focus, joint)] {
                    let key = format!("worn_{name}");
                    let shell = assets.procedural_avatars[&key]
                        .document
                        .with_wearer(&actor.document.wearer_body().unwrap())
                        .evaluate()
                        .unwrap()
                        .mesh;
                    let attachment = compiled
                        .evaluation
                        .mesh
                        .attachments
                        .iter()
                        .find(|a| a.id == key)
                        .unwrap();
                    assert_eq!(rig.joints[attachment.joint].id, joint);
                    for clip in ["idle", "walk", "attack"] {
                        let motion = Appearance::motion(&compiled, clip).unwrap();
                        for phase in [0., 0.25, 0.5, 0.75] {
                            let posed = compiled
                                .evaluation
                                .mesh
                                .posed(Some(motion), phase * motion.duration)
                                .unwrap();
                            let transform = posed.joints[attachment.joint]
                                .compose(rig.rest()[attachment.joint].inverse());
                            assert!(posed.attachments.contains_key(&key));
                            for part in &shell.parts {
                                let target =
                                    posed.mesh.parts.iter().find(|p| p.id == part.id).unwrap();
                                assert_eq!(part.texture, target.texture);
                                for (a, b) in part.vertices.iter().zip(&target.vertices) {
                                    let expected = transform.point(a.position);
                                    for k in 0..3 {
                                        assert!(
                                            (expected[k] - b.position[k]).abs() < 0.00004,
                                            "{race}/{name}/{clip}"
                                        );
                                    }
                                    assert_eq!(a.uv, b.uv);
                                }
                            }
                        }
                    }
                }
                assert!(
                    actor
                        .sprite(&entity, &assets, "walk", 0.3)
                        .unwrap()
                        .chunks_exact(4)
                        .any(|p| p[3] != 0)
                );
            }
        }
    }
    #[test]
    fn ruleset_shield_slot_resolves_to_the_rig_off_hand_anchor() {
        let mut assets = Assets::new();
        assets.rules =
            "[equipment.avatar_anchors]\nshield = 'off_hand'\nmain_hand = 'main_hand'\n".into();
        let mut entity = Entity::default();
        let shield = crate::Item {
            id: 123,
            ..Default::default()
        };
        entity.equipped.insert("shield".into(), shield);
        assert_eq!(
            Appearance::equipped_for_attachment(&entity, "off_hand", &assets)
                .unwrap()
                .id,
            123
        );
        assert!(Appearance::equipped_for_attachment(&entity, "main_hand", &assets).is_none());
    }
    #[test]
    fn torch_geometry_uses_active_state_and_flat_coin_previews_keep_the_rim() {
        let (assets, _, entity) = fixture();
        let templates = eldiron_ruleset::ruleset_item_templates_from_source(
            eldiron_ruleset::latest_official_ruleset(),
        )
        .unwrap();
        let mut torch = Item::default();
        crate::server::data::apply_item_data(
            &mut torch,
            &templates.iter().find(|t| t.id == "torch").unwrap().data,
        );
        for (active, model) in [(false, "torch_off"), (true, "torch"), (false, "torch_off")] {
            torch.set_attribute("active", Value::Bool(active));
            assert!(Arc::ptr_eq(
                Runtime::resolve_item(&torch, &assets).unwrap(),
                &assets.procedural_avatars[model]
            ));
        }
        for name in ["copper_coin", "silver_coin", "gold_coin"] {
            let model = &assets.procedural_avatars[name];
            let pixels = model.sprite(&entity, &assets, "idle", 0.).unwrap();
            let coordinates = pixels
                .chunks_exact(4)
                .enumerate()
                .filter(|(_, p)| p[3] != 0)
                .map(|(i, _)| (i % 64, i / 64))
                .collect::<Vec<_>>();
            assert!(coordinates.len() > 100, "{name}");
            assert!(
                coordinates
                    .iter()
                    .all(|(x, y)| *x > 0 && *x < 63 && *y > 0 && *y < 63),
                "{name}: clipped rim"
            );
        }
    }
    #[test]
    fn new_weapon_grips_follow_every_race_hand_in_every_clip() {
        use denrim_materials::geometry::rig::Pose;
        let (mut assets, human, mut entity) = fixture();
        assets
            .procedural_avatars
            .insert("humanoid".into(), Arc::new(human));
        let templates = eldiron_ruleset::ruleset_item_templates_from_source(
            eldiron_ruleset::latest_official_ruleset(),
        )
        .unwrap();
        for race in ["humanoid", "orc", "skeleton"] {
            let actor = &assets.procedural_avatars[race];
            let bare = actor.compile(0, &assets).unwrap();
            for weapon in ["novice_mace", "hunting_bow", "training_spear"] {
                entity.equipped.clear();
                let mut item = Item::default();
                crate::server::data::apply_item_data(
                    &mut item,
                    &templates.iter().find(|t| t.id == weapon).unwrap().data,
                );
                let model = Runtime::resolve_item(&item, &assets).unwrap();
                assert!(Arc::ptr_eq(model, &assets.procedural_avatars[weapon]));
                let source = model.document.evaluate().unwrap();
                let grip = Pose::from_frame(source.grip.as_ref().unwrap());
                entity.equipped.insert("main_hand".into(), item);
                let (compiled, _) = actor.compiled_for(&entity, &assets).unwrap();
                assert_eq!(compiled.height, bare.height);
                assert!(
                    bare.evaluation
                        .mesh
                        .rig
                        .as_ref()
                        .unwrap()
                        .joints
                        .iter()
                        .all(|j| compiled
                            .evaluation
                            .mesh
                            .rig
                            .as_ref()
                            .unwrap()
                            .index(&j.id)
                            .is_ok())
                );
                let attachment = compiled
                    .evaluation
                    .mesh
                    .attachments
                    .iter()
                    .find(|a| a.id == "sword")
                    .unwrap();
                let rig = compiled.evaluation.mesh.rig.as_ref().unwrap();
                assert_eq!(
                    rig.joints[attachment.joint].id,
                    if weapon == "hunting_bow" {
                        "left-hand"
                    } else {
                        "right-hand"
                    }
                );
                assert_eq!(attachment.grip, *source.grip.as_ref().unwrap());
                for clip in ["idle", "walk", "attack"] {
                    let motion = Appearance::motion(&compiled, clip).unwrap();
                    for phase in [0., 0.25, 0.5, 0.75, 1.] {
                        let posed = compiled
                            .evaluation
                            .mesh
                            .posed(Some(motion), phase * motion.duration)
                            .unwrap();
                        let socket = posed.joints[attachment.joint]
                            .compose(Pose::from_frame(&attachment.socket));
                        let transform = socket.compose(grip.inverse());
                        let actual_socket = posed.attachments["sword"];
                        for k in 0..3 {
                            assert!(
                                (actual_socket.translation[k] - socket.translation[k]).abs()
                                    < 0.00002
                            );
                        }
                        let item_motion = if weapon == "hunting_bow" && clip == "attack" {
                            if let Some(GeoValue::Motion(m)) = source.rows.get("bow-shot") {
                                Some(m)
                            } else {
                                None
                            }
                        } else {
                            None
                        };
                        let item_pose = source
                            .mesh
                            .posed(item_motion, item_motion.map_or(0., |m| phase * m.duration))
                            .unwrap();
                        for part in &item_pose.mesh.parts {
                            let target = posed.mesh.parts.iter().find(|p| p.id == part.id).unwrap();
                            assert_eq!(part.vertices.len(), target.vertices.len());
                            for (a, b) in part.vertices.iter().zip(&target.vertices) {
                                let expected = transform.point(a.position);
                                for k in 0..3 {
                                    assert!(
                                        (expected[k] - b.position[k]).abs() < 0.00003,
                                        "{race}/{weapon}/{clip}: detached vertex"
                                    );
                                }
                                assert_eq!(a.uv, b.uv);
                            }
                        }
                    }
                }
                if weapon != "novice_mace" {
                    let posed = compiled.evaluation.mesh.posed(None, 0.).unwrap();
                    let lo = posed
                        .mesh
                        .parts
                        .iter()
                        .filter(|p| source.mesh.parts.iter().any(|s| s.id == p.id))
                        .flat_map(|p| &p.vertices)
                        .map(|v| v.position[1])
                        .fold(f32::INFINITY, f32::min);
                    assert!(
                        lo > bare.floor + 0.08,
                        "{race}/{weapon}: carry pose enters the floor"
                    );
                }
                let front = actor.sprite(&entity, &assets, "idle", 0.).unwrap();
                entity.orientation = Vec2::new(1., 0.);
                assert_ne!(front, actor.sprite(&entity, &assets, "idle", 0.).unwrap());
                entity.orientation = Vec2::new(0., 1.);
            }
        }
    }
    #[test]
    fn orc_heavy_walk_plants_feet_and_clothing_tracks_the_sculpted_body() {
        let (assets, human, mut entity) = fixture();
        let actor = &assets.procedural_avatars["orc"];
        let c = actor.compile(0, &assets).unwrap();
        let h = human.compile(0, &assets).unwrap();
        assert!(
            Appearance::motion(&c, "idle").unwrap().duration
                > Appearance::motion(&h, "idle").unwrap().duration
        );
        let walk = Appearance::motion(&c, "walk").unwrap();
        assert!(walk.duration >= 1.3);
        let rig = c.evaluation.mesh.rig.as_ref().unwrap();
        let rest = rig.rest();
        let mut planted = [0; 2];
        let mut double_support = 0;
        for step in 0..32 {
            let pose = c
                .evaluation
                .mesh
                .posed(Some(walk), step as f32 / 32. * walk.duration)
                .unwrap();
            let mut support = 0;
            for (i, side) in ["left", "right"].into_iter().enumerate() {
                let at = rig.index(&format!("{side}-foot")).unwrap();
                let lift = pose.joints[at].translation[1] - rest[at].translation[1];
                assert!(lift >= -0.003 && lift < 0.06, "{side}/{step}: {lift}");
                if lift.abs() < 0.003 {
                    planted[i] += 1;
                    support += 1;
                }
            }
            if support == 2 {
                double_support += 1;
            }
            for name in ["left-boot", "right-boot"] {
                let foot = pose.mesh.parts.iter().find(|p| p.id == name).unwrap();
                assert!(
                    foot.vertices
                        .iter()
                        .all(|v| v.position[1] >= c.floor - 0.003)
                );
            }
        }
        assert!(planted.iter().all(|n| *n >= 20));
        assert!(double_support >= 6);
        let templates = eldiron_ruleset::ruleset_item_templates_from_source(
            eldiron_ruleset::latest_official_ruleset(),
        )
        .unwrap();
        let item = |name: &str| {
            let mut item = Item::default();
            crate::server::data::apply_item_data(
                &mut item,
                &templates.iter().find(|t| t.id == name).unwrap().data,
            );
            item
        };
        for garment in [
            "linen_shirt",
            "padded_armor",
            "leather_vest",
            "chain_shirt",
            "cleric_vestments",
        ] {
            entity.equipped.clear();
            for (slot, name) in [
                ("torso", garment),
                ("legs", "wool_trousers"),
                ("feet", "leather_shoes"),
                ("main_hand", "novice_mace"),
                ("off_hand", "round_shield"),
            ] {
                entity.equipped.insert(slot.into(), item(name));
            }
            let (dressed, _) = actor.compiled_for(&entity, &assets).unwrap();
            assert_eq!(dressed.evaluation.mesh.rig, c.evaluation.mesh.rig);
            for name in [
                "body-torso",
                "left-arm",
                "right-arm",
                "left-sleeve",
                "right-sleeve",
                "left-boot",
                "right-boot",
            ] {
                let bare = c
                    .evaluation
                    .mesh
                    .parts
                    .iter()
                    .find(|p| p.id == name)
                    .unwrap();
                let shell = dressed
                    .evaluation
                    .mesh
                    .parts
                    .iter()
                    .find(|p| p.id == name)
                    .unwrap();
                assert_eq!(bare.vertices.len(), shell.vertices.len());
                assert_ne!(bare.texture, shell.texture);
                for (a, b) in bare.vertices.iter().zip(&shell.vertices) {
                    assert_eq!(a.skin, b.skin);
                    assert_eq!(a.uv, b.uv);
                    let offset = (0..3)
                        .map(|k| (b.position[k] - a.position[k]) * a.normal[k])
                        .sum::<f32>();
                    assert!(
                        offset >= 0.002 && offset <= 0.022,
                        "{garment}/{name}: shell crossed body"
                    );
                }
            }
            for clip in ["idle", "walk", "attack"] {
                let m = Appearance::motion(&dressed, clip).unwrap();
                for phase in [0., 0.25, 0.5, 0.75, 1.] {
                    let posed = dressed
                        .evaluation
                        .mesh
                        .posed(Some(m), phase * m.duration)
                        .unwrap();
                    assert!(
                        posed.attachments.contains_key("sword")
                            && posed.attachments.contains_key("shield")
                    );
                }
            }
        }
    }
    #[test]
    fn creature_recipes_share_clothes_sockets_and_animation_in_both_renderers() {
        let (assets, human, mut entity) = fixture();
        let human_sprite = human.sprite(&entity, &assets, "idle", 0.).unwrap();
        let templates = eldiron_ruleset::ruleset_item_templates_from_source(
            eldiron_ruleset::latest_official_ruleset(),
        )
        .unwrap();
        for name in ["orc", "skeleton"] {
            let appearance = &assets.procedural_avatars[name];
            let doc = Document::load(&appearance.source).unwrap();
            assert_eq!(Document::load(&doc.save().unwrap()).unwrap(), doc);
            entity.equipped.clear();
            let bare = appearance.compile(0, &assets).unwrap();
            assert_eq!(bare.evaluation.mesh.rig.as_ref().unwrap().joints.len(), 17);
            assert!(bare.evaluation.mesh.vertex_count() < 50_000);
            let front = appearance.sprite(&entity, &assets, "idle", 0.).unwrap();
            assert_ne!(front, human_sprite);
            assert!(front.chunks_exact(4).filter(|p| p[3] > 0).count() > 100);
            if name == "skeleton" {
                assert_eq!(
                    bare.evaluation
                        .mesh
                        .parts
                        .iter()
                        .filter(|p| p.id.starts_with("rib-"))
                        .count(),
                    6
                );
                for (part, mat) in bare.evaluation.mesh.parts.iter().zip(&bare.materials) {
                    if part.texture.as_ref().unwrap().source == "ruleset:material/fitting-guide" {
                        assert!(mat.texture.data.chunks_exact(4).all(|p| p[3] == 0));
                    }
                }
            } else {
                assert!(
                    bare.evaluation
                        .mesh
                        .parts
                        .iter()
                        .any(|p| p.id == "tusk-left")
                );
            }
            for (slot, item_name) in [
                ("torso", "chain_shirt"),
                ("legs", "wool_trousers"),
                ("feet", "leather_shoes"),
                ("main_hand", "training_sword"),
                ("off_hand", "round_shield"),
            ] {
                let mut item = Item::default();
                crate::server::data::apply_item_data(
                    &mut item,
                    &templates.iter().find(|t| t.id == item_name).unwrap().data,
                );
                entity.equipped.insert(slot.into(), item);
            }
            let (worn, _) = appearance.compiled_for(&entity, &assets).unwrap();
            assert_eq!(worn.evaluation.mesh.rig, bare.evaluation.mesh.rig);
            assert_eq!(worn.evaluation.mesh.attachments.len(), 2);
            assert_eq!(worn.height, bare.height);
            assert_ne!(
                appearance.sprite(&entity, &assets, "idle", 0.).unwrap(),
                front
            );
            for clip in ["idle", "walk", "attack"] {
                let pose = Appearance::pose(&worn, clip, 0.3).unwrap();
                assert!(
                    pose.parts.iter().flat_map(|p| &p.vertices).all(|v| v
                        .position
                        .iter()
                        .chain(&v.normal)
                        .all(|x| x.is_finite()))
                );
            }
            for direction in 0..8 {
                let angle = (direction as f32 * 45.).to_radians();
                entity.orientation = Vec2::new(angle.sin(), angle.cos());
                let rgba = appearance.sprite(&entity, &assets, "walk", 0.3).unwrap();
                assert!(rgba.chunks_exact(4).filter(|p| p[3] > 0).all(|p| {
                    assets
                        .palette
                        .colors
                        .iter()
                        .flatten()
                        .any(|c| c.to_u8_array()[..3] == p[..3])
                }));
            }
            entity.orientation = Vec2::new(0., 1.);
        }
    }
    #[test]
    fn ruleset_clothes_change_shared_materials_and_fit_without_changing_the_rig() {
        let (assets, appearance, mut entity) = fixture();
        let baseline = appearance.compile(0, &assets).unwrap();
        let templates = eldiron_ruleset::ruleset_item_templates_from_source(
            eldiron_ruleset::latest_official_ruleset(),
        )
        .unwrap();
        let mut pixels = Vec::new();
        let mut keys = std::collections::BTreeSet::new();
        for name in [
            "linen_shirt",
            "padded_armor",
            "leather_vest",
            "chain_shirt",
            "cleric_vestments",
        ] {
            let template = templates.iter().find(|t| t.id == name).unwrap();
            let mut item = Item::default();
            crate::server::data::apply_item_data(&mut item, &template.data);
            entity.equipped.insert("torso".into(), item);
            let (compiled, key) = appearance.compiled_for(&entity, &assets).unwrap();
            assert!(keys.insert(key));
            assert_eq!(compiled.height, baseline.height);
            assert_eq!(compiled.floor, baseline.floor);
            assert_eq!(compiled.evaluation.mesh.rig, baseline.evaluation.mesh.rig);
            if name == "cleric_vestments" {
                assert_eq!(
                    compiled.evaluation.mesh.parts.len(),
                    baseline.evaluation.mesh.parts.len() + 4
                );
                let posed = Appearance::pose(&compiled, "walk", 0.3).unwrap();
                let panel = posed
                    .parts
                    .iter()
                    .find(|p| p.id == "vestment-left-front")
                    .unwrap();
                let bind = compiled
                    .evaluation
                    .mesh
                    .parts
                    .iter()
                    .find(|p| p.id == panel.id)
                    .unwrap();
                assert!(
                    panel
                        .vertices
                        .iter()
                        .zip(&bind.vertices)
                        .any(|(a, b)| a.position != b.position)
                );
            }
            let torso = compiled
                .evaluation
                .mesh
                .parts
                .iter()
                .find(|p| p.id == "body-torso")
                .unwrap();
            let old = baseline
                .evaluation
                .mesh
                .parts
                .iter()
                .find(|p| p.id == "body-torso")
                .unwrap();
            assert_ne!(torso.texture, old.texture);
            assert_ne!(torso.vertices[0].position, old.vertices[0].position);
            assert!(
                torso
                    .vertices
                    .iter()
                    .zip(&old.vertices)
                    .all(|(a, b)| a.uv == b.uv && a.skin == b.skin)
            );
            let hand = compiled
                .evaluation
                .mesh
                .parts
                .iter()
                .find(|p| p.id == "left-palm")
                .unwrap();
            assert_eq!(
                hand,
                baseline
                    .evaluation
                    .mesh
                    .parts
                    .iter()
                    .find(|p| p.id == "left-palm")
                    .unwrap()
            );
            let sprite = appearance.sprite(&entity, &assets, "walk", 0.3).unwrap();
            assert!(
                pixels.iter().all(|old| old != &sprite),
                "{name} must have a distinct appearance"
            );
            pixels.push(sprite);
            for clip in ["idle", "walk", "attack"] {
                let pose = Appearance::pose(&compiled, clip, 0.3).unwrap();
                assert!(
                    pose.parts.iter().flat_map(|p| &p.vertices).all(|v| v
                        .position
                        .iter()
                        .chain(&v.normal)
                        .all(|x| x.is_finite()))
                );
            }
        }
        for (slot, name, part) in [
            ("legs", "wool_trousers", "body-pelvis"),
            ("feet", "leather_shoes", "left-boot"),
        ] {
            let mut item = Item::default();
            crate::server::data::apply_item_data(
                &mut item,
                &templates.iter().find(|t| t.id == name).unwrap().data,
            );
            entity.equipped.insert(slot.into(), item);
            let (compiled, _) = appearance.compiled_for(&entity, &assets).unwrap();
            assert_ne!(
                compiled
                    .evaluation
                    .mesh
                    .parts
                    .iter()
                    .find(|p| p.id == part)
                    .unwrap()
                    .texture,
                baseline
                    .evaluation
                    .mesh
                    .parts
                    .iter()
                    .find(|p| p.id == part)
                    .unwrap()
                    .texture
            );
        }
        entity.equipped.clear();
        assert_eq!(appearance.compiled_for(&entity, &assets).unwrap().1, 0);
    }
    #[test]
    fn ruleset_recipe_generates_distinct_palette_sprites_and_equipped_meshes() {
        let (assets, appearance, mut entity) = fixture();
        let bare = appearance.compile(0, &assets).unwrap();
        let armed = appearance.compile(3, &assets).unwrap();
        assert_eq!(bare.evaluation.mesh.parts.len(), 29);
        for (part, material) in bare.evaluation.mesh.parts.iter().zip(&bare.materials) {
            let spec = part
                .texture
                .as_ref()
                .expect("Every body region needs a ruleset MZ surface");
            assert!(spec.source.starts_with("ruleset:material/"));
            let expected = &assets.ruleset_materials[&spec.source];
            assert_eq!(material.texture.data, expected.data);
            assert_eq!(Some(&material.packed), expected.data_ext.as_ref());
        }
        let posed = Appearance::pose(&bare, "walk", 0.3).unwrap();
        for (bind, posed) in bare.evaluation.mesh.parts.iter().zip(&posed.parts) {
            assert!(
                bind.vertices
                    .iter()
                    .zip(&posed.vertices)
                    .all(|(a, b)| a.uv == b.uv),
                "UVs must move with the skin"
            );
        }

        let attack_duration = Appearance::motion(&bare, "attack").unwrap().duration;
        let attack_mid = Appearance::animation_seconds(&bare, &entity, "attack", 0.175);
        assert!(
            (attack_mid - attack_duration * 0.5).abs() < 0.001,
            "the complete attack must fit the gameplay attack window"
        );
        assert!(armed.evaluation.mesh.vertex_count() > bare.evaluation.mesh.vertex_count());
        assert_eq!((bare.height, bare.floor), (armed.height, armed.floor));
        let front = appearance.sprite(&entity, &assets, "idle", 0.).unwrap();
        assert!(front.chunks_exact(4).filter(|p| p[3] > 0).count() > 100);
        assert!(front.chunks_exact(4).any(|p| p[3] == 0));
        for p in front.chunks_exact(4).filter(|p| p[3] > 0) {
            assert!(
                assets
                    .palette
                    .colors
                    .iter()
                    .flatten()
                    .any(|c| &c.to_u8_array()[..3] == &p[..3])
            );
        }
        entity.orientation = Vec2::new(1., 0.);
        assert_ne!(
            front,
            appearance.sprite(&entity, &assets, "idle", 0.).unwrap()
        );
        assert_ne!(
            appearance.sprite(&entity, &assets, "walk", 0.).unwrap(),
            appearance.sprite(&entity, &assets, "walk", 0.3).unwrap()
        );
        let mut sword = Item::default();
        sword.item_type = "Longsword".into();
        entity.equipped.insert("main_hand".into(), sword);
        assert_eq!(appearance.equipment_mask(&entity, &assets), 1);
        let mut axe = Item::default();
        axe.item_type = "Axe".into();
        entity.equipped.insert("main_hand".into(), axe);
        assert_eq!(
            appearance.equipment_mask(&entity, &assets),
            0,
            "unsupported equipment must not become a sword"
        );
        // Attack is non-looping: its final sprite is retained, rather than wrapping.
        assert_eq!(
            appearance.sprite(&entity, &assets, "attack", 100.).unwrap(),
            appearance.sprite(&entity, &assets, "attack", 101.).unwrap()
        );
    }
    #[test]
    fn attack_retrigger_restarts_and_explicit_bitmaps_do_not_resolve_as_models() {
        let (mut assets, appearance, mut entity) = fixture();
        assets
            .procedural_avatars
            .insert("humanoid".into(), Arc::new(appearance));
        assert!(Runtime::resolve(&entity, &assets).is_some());
        entity.set_attribute("avatar", Value::Str("custom_bitmap".into()));
        assert!(Runtime::resolve(&entity, &assets).is_none());
        entity.set_attribute("avatar_animation", Value::Str("Attack".into()));
        let mut runtime = Runtime::default();
        let id = GeoId::Unknown(1);
        runtime.time = 1.;
        assert_eq!(runtime.phase(id, &entity).1, 0.);
        runtime.time = 1.4;
        assert!((runtime.phase(id, &entity).1 - 0.4).abs() < 0.001);
        entity.set_attribute("avatar_attack_serial", Value::Int(1));
        assert_eq!(runtime.phase(id, &entity).1, 0.);
    }
}
