use crate::{
    Assets, Avatar, AvatarBuildRequest, AvatarBuilder, AvatarDirection, AvatarMarkerColors, Entity,
    Item, Value,
};
use rustc_hash::{FxHashMap, FxHashSet};
use scenevm::{Atom, GeoId, SceneVM};
use theframework::prelude::*;

struct CachedAvatarFrames {
    frames: FxHashMap<(String, AvatarDirection, usize), (u32, Vec<u8>)>,
    last_uploaded: Option<(String, AvatarDirection, usize)>,
}

#[derive(Default)]
pub struct AvatarRuntimeBuilder {
    // Latch for update_avatar=true edge detection.
    avatar_rebuild_latch: FxHashSet<GeoId>,
    // Per-entity built avatar frames (all anims/perspectives/frames).
    avatar_frame_cache: FxHashMap<GeoId, CachedAvatarFrames>,
}

impl AvatarRuntimeBuilder {
    pub fn find_avatar_for_entity<'a>(entity: &Entity, assets: &'a Assets) -> Option<&'a Avatar> {
        if let Some(avatar_id) = entity.attributes.get_id("avatar_id") {
            if let Some(avatar) = assets.avatars.get(&avatar_id.to_string()) {
                return Some(avatar);
            }
            for avatar in assets.avatars.values() {
                if avatar.id == avatar_id {
                    return Some(avatar);
                }
            }
        }
        if let Some(name) = entity.attributes.get_str("avatar") {
            return assets.avatars.get(name);
        }
        None
    }

    fn avatar_direction_from_entity(entity: &Entity) -> AvatarDirection {
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

    fn marker_color_from_index(assets: &Assets, idx: i32, default: [u8; 4]) -> [u8; 4] {
        if idx < 0 {
            return default;
        }
        let i = idx as usize;
        if i < assets.palette.colors.len() {
            if let Some(col) = &assets.palette[i] {
                return col.to_u8_array();
            }
        }
        default
    }

    fn marker_color(
        entity: &Entity,
        assets: &Assets,
        value_key: &str,
        color_key: &str,
        index_key: &str,
        default: [u8; 4],
    ) -> [u8; 4] {
        if let Some(Value::Color(c)) = entity.attributes.get(value_key) {
            return c.to_u8_array();
        }
        if let Some(hex) = entity.attributes.get_str(color_key) {
            return TheColor::from_hex(hex).to_u8_array();
        }
        if let Some(idx) = entity.attributes.get_int(index_key) {
            return Self::marker_color_from_index(assets, idx, default);
        }
        default
    }

    fn equipped_slot_color(item: &Item, assets: &Assets, default: [u8; 4]) -> [u8; 4] {
        if let Some(Value::Color(c)) = item.attributes.get("avatar_color") {
            return c.to_u8_array();
        }
        if let Some(hex) = item.attributes.get_str("avatar_color_hex") {
            return TheColor::from_hex(hex).to_u8_array();
        }
        if let Some(idx) = item.attributes.get_int("avatar_color_index") {
            return Self::marker_color_from_index(assets, idx, default);
        }
        default
    }

    fn slot_override_color(
        entity: &Entity,
        assets: &Assets,
        slot_names: &[&str],
        default: [u8; 4],
    ) -> [u8; 4] {
        for slot in slot_names {
            if let Some(item) = entity.equipped.get(*slot) {
                return Self::equipped_slot_color(item, assets, default);
            }
        }
        default
    }

    fn marker_colors_for_entity(entity: &Entity, assets: &Assets) -> AvatarMarkerColors {
        let defaults = AvatarMarkerColors::default();
        let light_skin = Self::marker_color(
            entity,
            assets,
            "avatar_skin_light",
            "light_skin_color",
            "light_skin_index",
            defaults.skin_light,
        );
        AvatarMarkerColors {
            // Base/default marker color for all slots is light_skin.
            skin_light: light_skin,
            skin_dark: Self::marker_color(
                entity,
                assets,
                "avatar_skin_dark",
                "dark_skin_color",
                "dark_skin_index",
                light_skin,
            ),
            torso: Self::slot_override_color(
                entity,
                assets,
                &["torso", "chest", "armor"],
                Self::marker_color(
                    entity,
                    assets,
                    "avatar_torso",
                    "torso_color",
                    "torso_index",
                    light_skin,
                ),
            ),
            legs: Self::slot_override_color(
                entity,
                assets,
                &["legs", "pants"],
                Self::marker_color(
                    entity,
                    assets,
                    "avatar_legs",
                    "legs_color",
                    "legs_index",
                    light_skin,
                ),
            ),
            hair: Self::slot_override_color(
                entity,
                assets,
                &["head", "helmet", "hair"],
                Self::marker_color(
                    entity,
                    assets,
                    "avatar_hair",
                    "hair_color",
                    "hair_index",
                    light_skin,
                ),
            ),
            eyes: Self::slot_override_color(
                entity,
                assets,
                &["head", "helmet", "eyes"],
                Self::marker_color(
                    entity,
                    assets,
                    "avatar_eyes",
                    "eyes_color",
                    "eyes_index",
                    light_skin,
                ),
            ),
            hands: Self::slot_override_color(
                entity,
                assets,
                &["hands", "gloves"],
                Self::marker_color(
                    entity,
                    assets,
                    "avatar_hands",
                    "hands_color",
                    "hands_index",
                    light_skin,
                ),
            ),
            feet: Self::slot_override_color(
                entity,
                assets,
                &["feet", "boots"],
                Self::marker_color(
                    entity,
                    assets,
                    "avatar_feet",
                    "feet_color",
                    "feet_index",
                    light_skin,
                ),
            ),
        }
    }

    fn resolve_avatar_selection<'a>(
        avatar: &'a Avatar,
        animation_name: Option<&str>,
        direction: AvatarDirection,
    ) -> Option<(&'a str, AvatarDirection)> {
        let anim = animation_name
            .and_then(|name| {
                avatar
                    .animations
                    .iter()
                    .find(|a| a.name.eq_ignore_ascii_case(name))
            })
            .or_else(|| avatar.animations.first())?;

        let perspective = anim
            .perspectives
            .iter()
            .find(|p| p.direction == direction)
            .or_else(|| {
                anim.perspectives
                    .iter()
                    .find(|p| p.direction == AvatarDirection::Front)
            })
            .or_else(|| anim.perspectives.first())?;

        Some((anim.name.as_str(), perspective.direction))
    }

    fn rebuild_entity_avatar_cache(
        &mut self,
        vm: &mut SceneVM,
        entity: &Entity,
        avatar: &Avatar,
        assets: &Assets,
        geo_id: GeoId,
    ) -> bool {
        let markers = Self::marker_colors_for_entity(entity, assets);
        let mut frames: FxHashMap<(String, AvatarDirection, usize), (u32, Vec<u8>)> =
            FxHashMap::default();

        for anim in &avatar.animations {
            for perspective in &anim.perspectives {
                let frame_count = perspective.frames.len().max(1);
                for frame_index in 0..frame_count {
                    if let Some(out) = AvatarBuilder::build_current_stub(AvatarBuildRequest {
                        avatar,
                        animation_name: Some(anim.name.as_str()),
                        direction: perspective.direction,
                        frame_index,
                        marker_colors: markers,
                    }) {
                        frames.insert(
                            (anim.name.clone(), perspective.direction, frame_index),
                            (out.size, out.rgba),
                        );
                    }
                }
            }
        }

        if frames.is_empty() {
            self.avatar_frame_cache.remove(&geo_id);
            vm.execute(Atom::RemoveAvatarBillboardData { id: geo_id });
            return false;
        }

        self.avatar_frame_cache.insert(
            geo_id,
            CachedAvatarFrames {
                frames,
                last_uploaded: None,
            },
        );
        println!("rebuilt avatar entity {} cache", entity.id);
        true
    }

    pub fn ensure_entity_avatar_uploaded(
        &mut self,
        vm: &mut SceneVM,
        entity: &Entity,
        avatar: &Avatar,
        assets: &Assets,
        frame_index: usize,
        geo_id: GeoId,
    ) -> bool {
        let update_avatar = entity.attributes.get_bool_default("update_avatar", false);
        let needs_rebuild_edge = if update_avatar {
            self.avatar_rebuild_latch.insert(geo_id)
        } else {
            self.avatar_rebuild_latch.remove(&geo_id);
            false
        };
        let cache_missing = !self.avatar_frame_cache.contains_key(&geo_id);

        if (needs_rebuild_edge || cache_missing)
            && !self.rebuild_entity_avatar_cache(vm, entity, avatar, assets, geo_id)
        {
            return false;
        }

        let direction = Self::avatar_direction_from_entity(entity);
        let animation_name = entity.attributes.get_str("avatar_animation");
        let Some((anim_name, persp_dir)) =
            Self::resolve_avatar_selection(avatar, animation_name, direction)
        else {
            return false;
        };
        let anim = avatar
            .animations
            .iter()
            .find(|a| a.name.eq_ignore_ascii_case(anim_name))
            .or_else(|| avatar.animations.first());
        let Some(anim) = anim else {
            return false;
        };
        let frame_count = anim
            .perspectives
            .iter()
            .find(|p| p.direction == persp_dir)
            .or_else(|| anim.perspectives.first())
            .map(|p| p.frames.len().max(1))
            .unwrap_or(1);
        let speed = if anim.speed.is_finite() {
            anim.speed.max(0.01)
        } else {
            1.0
        };
        // speed is a time scale: 1.0 normal, >1.0 slower, <1.0 faster.
        let scaled_frame = (frame_index as f32 / speed).floor() as usize;
        let frame_idx = scaled_frame % frame_count;
        let key = (anim_name.to_string(), persp_dir, frame_idx);

        let Some(cache) = self.avatar_frame_cache.get_mut(&geo_id) else {
            return false;
        };
        let Some((size, rgba)) = cache.frames.get(&key) else {
            return false;
        };

        if cache.last_uploaded.as_ref() != Some(&key) {
            println!(
                "avatar entity {} -> anim='{}' perspective={:?}",
                entity.id, anim_name, persp_dir
            );
            vm.execute(Atom::SetAvatarBillboardData {
                id: geo_id,
                size: *size,
                rgba: rgba.clone(),
            });
            cache.last_uploaded = Some(key);
        }
        true
    }

    pub fn remove_stale_avatars(&mut self, vm: &mut SceneVM, active_avatar_geo: &FxHashSet<GeoId>) {
        let stale: Vec<GeoId> = self
            .avatar_frame_cache
            .keys()
            .copied()
            .filter(|id| !active_avatar_geo.contains(id))
            .collect();
        for id in stale {
            self.avatar_frame_cache.remove(&id);
            self.avatar_rebuild_latch.remove(&id);
            vm.execute(Atom::RemoveAvatarBillboardData { id });
        }
    }
}
