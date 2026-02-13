use crate::{
    Assets, Avatar, AvatarBuildOutput, AvatarBuildRequest, AvatarBuilder, AvatarDirection,
    AvatarMarkerColors, Entity, Item, PixelSource, Value,
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
    pub fn build_preview_for_entity(
        entity: &Entity,
        avatar: &Avatar,
        assets: &Assets,
        animation_name: Option<&str>,
        direction: AvatarDirection,
        frame_index: usize,
    ) -> Option<AvatarBuildOutput> {
        let markers = Self::marker_colors_for_entity(entity, assets);
        let anim_name = animation_name.or_else(|| entity.attributes.get_str("avatar_animation"));
        let Some((resolved_anim, resolved_dir)) =
            Self::resolve_avatar_selection(avatar, anim_name, direction)
        else {
            return None;
        };
        let mut out = AvatarBuilder::build_current_stub(AvatarBuildRequest {
            avatar,
            animation_name: Some(resolved_anim),
            direction: resolved_dir,
            frame_index,
            marker_colors: markers,
        })?;
        let (main_anchor, off_anchor) =
            Self::frame_anchors(avatar, resolved_anim, resolved_dir, frame_index);
        if resolved_dir == AvatarDirection::Back {
            let size = out.size as usize;
            let mut composed = vec![0_u8; size * size * 4];
            let mut overlay_out = AvatarBuildOutput {
                size: out.size,
                rgba: composed,
            };
            Self::compose_weapon_overlay(
                &mut overlay_out,
                entity,
                assets,
                resolved_dir,
                frame_index,
                main_anchor,
                off_anchor,
            );
            composed = overlay_out.rgba;
            Self::alpha_blit_rgba(&mut composed, size, size, &out.rgba, size, size, 0, 0);
            out.rgba = composed;
        } else {
            Self::compose_weapon_overlay(
                &mut out,
                entity,
                assets,
                resolved_dir,
                frame_index,
                main_anchor,
                off_anchor,
            );
        }
        Some(out)
    }

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

    fn marker_color_for_attrs(
        attrs: &crate::ValueContainer,
        assets: &Assets,
        value_key: Option<&str>,
        color_key: &str,
        index_key: &str,
        default: [u8; 4],
    ) -> [u8; 4] {
        if let Some(value_key) = value_key
            && let Some(Value::Color(c)) = attrs.get(value_key)
        {
            return c.to_u8_array();
        }
        if let Some(hex) = attrs.get_str(color_key) {
            return TheColor::from_hex(hex).to_u8_array();
        }
        if let Some(idx) = attrs.get_int(index_key) {
            return Self::marker_color_from_index(assets, idx, default);
        }
        default
    }

    fn marker_color(entity: &Entity, assets: &Assets, key: &str, default: [u8; 4]) -> [u8; 4] {
        Self::marker_color_for_attrs(
            &entity.attributes,
            assets,
            Some(&format!("avatar_{key}")),
            &format!("{key}_color"),
            &format!("{key}_index"),
            default,
        )
    }

    fn marker_color_from_item(item: &Item, assets: &Assets, key: &str) -> Option<[u8; 4]> {
        if let Some(Value::Color(c)) = item.attributes.get(&format!("avatar_{key}")) {
            return Some(c.to_u8_array());
        }
        if let Some(Value::Color(c)) = item.attributes.get(key) {
            return Some(c.to_u8_array());
        }
        if let Some(hex) = item.attributes.get_str(&format!("{key}_color")) {
            return Some(TheColor::from_hex(hex).to_u8_array());
        }
        if let Some(idx) = item.attributes.get_int(&format!("{key}_index")) {
            return Some(Self::marker_color_from_index(assets, idx, [0, 0, 0, 255]));
        }
        None
    }

    fn marker_colors_for_entity(entity: &Entity, assets: &Assets) -> AvatarMarkerColors {
        let defaults = AvatarMarkerColors::default();
        let light_skin = Self::marker_color(entity, assets, "light_skin", defaults.skin_light);
        let mut colors = AvatarMarkerColors {
            // Base/default marker color for all slots is light_skin.
            skin_light: light_skin,
            skin_dark: Self::marker_color(entity, assets, "dark_skin", light_skin),
            torso: Self::marker_color(entity, assets, "torso", light_skin),
            legs: Self::marker_color(entity, assets, "legs", light_skin),
            hair: Self::marker_color(entity, assets, "hair", light_skin),
            eyes: Self::marker_color(entity, assets, "eyes", light_skin),
            hands: Self::marker_color(entity, assets, "hands", light_skin),
            feet: Self::marker_color(entity, assets, "feet", light_skin),
        };

        // Equipped items can override any marker channel (e.g. torso_color/index on armor).
        for item in entity.equipped.values() {
            if let Some(c) = Self::marker_color_from_item(item, assets, "light_skin") {
                colors.skin_light = c;
            }
            if let Some(c) = Self::marker_color_from_item(item, assets, "dark_skin") {
                colors.skin_dark = c;
            }
            if let Some(c) = Self::marker_color_from_item(item, assets, "torso") {
                colors.torso = c;
            }
            if let Some(c) = Self::marker_color_from_item(item, assets, "legs") {
                colors.legs = c;
            }
            if let Some(c) = Self::marker_color_from_item(item, assets, "hair") {
                colors.hair = c;
            }
            if let Some(c) = Self::marker_color_from_item(item, assets, "eyes") {
                colors.eyes = c;
            }
            if let Some(c) = Self::marker_color_from_item(item, assets, "hands") {
                colors.hands = c;
            }
            if let Some(c) = Self::marker_color_from_item(item, assets, "feet") {
                colors.feet = c;
            }
        }
        colors
    }

    fn item_tile_id_for_direction(item: &Item, direction: AvatarDirection) -> Option<Uuid> {
        let suffix = match direction {
            AvatarDirection::Front => "front",
            AvatarDirection::Back => "back",
            AvatarDirection::Left => "left",
            AvatarDirection::Right => "right",
        };
        let directional_keys = [
            format!("tile_id_{suffix}"),
            format!("rig_tile_id_{suffix}"),
            format!("tile_{suffix}"),
        ];
        let generic_keys = ["tile_id", "rig_tile_id", "tile"];

        for key in directional_keys
            .iter()
            .map(|s| s.as_str())
            .chain(generic_keys.iter().copied())
        {
            if let Some(id) = item.attributes.get_id(key) {
                return Some(id);
            }
            if let Some(PixelSource::TileId(id)) = item.attributes.get_source(key) {
                return Some(*id);
            }
            if let Some(raw) = item.attributes.get_str(key)
                && let Ok(id) = Uuid::parse_str(raw)
            {
                return Some(id);
            }
        }
        None
    }

    fn weapon_order_for_direction(direction: AvatarDirection) -> [(&'static str, bool); 2] {
        match direction {
            AvatarDirection::Front => [("off_hand", false), ("main_hand", true)],
            AvatarDirection::Back => [("main_hand", true), ("off_hand", false)],
            AvatarDirection::Left => [("off_hand", false), ("main_hand", true)],
            AvatarDirection::Right => [("main_hand", true), ("off_hand", false)],
        }
    }

    fn find_equipped_for_slot<'a>(entity: &'a Entity, canonical_slot: &str) -> Option<&'a Item> {
        let aliases: &[&str] = match canonical_slot {
            "main_hand" => &[
                "main_hand",
                "mainhand",
                "weapon",
                "weapon_main",
                "hand_main",
            ],
            "off_hand" => &["off_hand", "offhand", "weapon_off", "hand_off", "shield"],
            _ => &[],
        };
        for alias in aliases {
            if let Some(item) = entity.equipped.get(*alias) {
                return Some(item);
            }
        }
        None
    }

    fn scaled_texture_rgba(
        texture: &crate::Texture,
        scale: f32,
    ) -> Option<(Vec<u8>, usize, usize)> {
        if texture.width == 0 || texture.height == 0 {
            return None;
        }
        let scale = if scale.is_finite() {
            scale.max(0.01)
        } else {
            1.0
        };
        let out_w = ((texture.width as f32) * scale).round().max(1.0) as usize;
        let out_h = ((texture.height as f32) * scale).round().max(1.0) as usize;
        let mut out = vec![0_u8; out_w * out_h * 4];
        for y in 0..out_h {
            let src_y = ((y as f32) / scale)
                .floor()
                .clamp(0.0, (texture.height.saturating_sub(1)) as f32)
                as usize;
            for x in 0..out_w {
                let src_x = ((x as f32) / scale)
                    .floor()
                    .clamp(0.0, (texture.width.saturating_sub(1)) as f32)
                    as usize;
                let src_i = (src_y * texture.width + src_x) * 4;
                let dst_i = (y * out_w + x) * 4;
                out[dst_i..dst_i + 4].copy_from_slice(&texture.data[src_i..src_i + 4]);
            }
        }
        Some((out, out_w, out_h))
    }

    fn alpha_blit_rgba(
        dst: &mut [u8],
        dst_w: usize,
        dst_h: usize,
        src: &[u8],
        src_w: usize,
        src_h: usize,
        dst_x: i32,
        dst_y: i32,
    ) {
        for sy in 0..src_h as i32 {
            let dy = dst_y + sy;
            if dy < 0 || dy >= dst_h as i32 {
                continue;
            }
            for sx in 0..src_w as i32 {
                let dx = dst_x + sx;
                if dx < 0 || dx >= dst_w as i32 {
                    continue;
                }
                let sidx = ((sy as usize) * src_w + (sx as usize)) * 4;
                let didx = ((dy as usize) * dst_w + (dx as usize)) * 4;
                let sa = src[sidx + 3] as f32 / 255.0;
                if sa <= 0.0 {
                    continue;
                }
                let da = dst[didx + 3] as f32 / 255.0;
                let out_a = sa + da * (1.0 - sa);
                if out_a <= 0.0 {
                    continue;
                }
                for c in 0..3 {
                    let sc = src[sidx + c] as f32 / 255.0;
                    let dc = dst[didx + c] as f32 / 255.0;
                    let out_c = (sc * sa + dc * da * (1.0 - sa)) / out_a;
                    dst[didx + c] = (out_c.clamp(0.0, 1.0) * 255.0) as u8;
                }
                dst[didx + 3] = (out_a.clamp(0.0, 1.0) * 255.0) as u8;
            }
        }
    }

    fn flip_rgba_horizontal(src: &[u8], w: usize, h: usize) -> Vec<u8> {
        let mut out = vec![0_u8; src.len()];
        for y in 0..h {
            for x in 0..w {
                let src_x = w - 1 - x;
                let src_i = (y * w + src_x) * 4;
                let dst_i = (y * w + x) * 4;
                out[dst_i..dst_i + 4].copy_from_slice(&src[src_i..src_i + 4]);
            }
        }
        out
    }

    fn compose_weapon_overlay(
        out: &mut AvatarBuildOutput,
        entity: &Entity,
        assets: &Assets,
        direction: AvatarDirection,
        frame_index: usize,
        main_anchor: Option<(i16, i16)>,
        off_anchor: Option<(i16, i16)>,
    ) {
        for (slot, is_main) in Self::weapon_order_for_direction(direction) {
            let anchor = if is_main { main_anchor } else { off_anchor };
            let Some(anchor) = anchor else {
                continue;
            };
            let Some(item) = Self::find_equipped_for_slot(entity, slot) else {
                continue;
            };
            let Some(tile_id) = Self::item_tile_id_for_direction(item, direction) else {
                continue;
            };
            let Some(tile) = assets.tiles.get(&tile_id) else {
                continue;
            };
            if tile.textures.is_empty() {
                continue;
            }
            let tex = &tile.textures[frame_index % tile.textures.len()];
            let scale = item.attributes.get_float_default("rig_scale", 1.0);
            let mut pivot = Self::item_rig_pivot(item);
            let Some((scaled, sw, sh)) = Self::scaled_texture_rgba(tex, scale) else {
                continue;
            };
            let scaled = if direction == AvatarDirection::Left {
                pivot[0] = 1.0 - pivot[0];
                Self::flip_rgba_horizontal(&scaled, sw, sh)
            } else {
                scaled
            };
            let px = (pivot[0].clamp(0.0, 1.0) * (sw as f32 - 1.0)).round() as i32;
            let py = (pivot[1].clamp(0.0, 1.0) * (sh as f32 - 1.0)).round() as i32;
            let dst_x = anchor.0 as i32 - px;
            let dst_y = anchor.1 as i32 - py;
            let size = out.size as usize;
            Self::alpha_blit_rgba(&mut out.rgba, size, size, &scaled, sw, sh, dst_x, dst_y);
        }
    }

    fn item_rig_pivot(item: &Item) -> [f32; 2] {
        if let Some(v) = item.attributes.get_vec2("rig_pivot") {
            return v;
        }
        if let Some(raw) = item.attributes.get_str("rig_pivot") {
            let parts: Vec<&str> = raw
                .split(',')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect();
            if parts.len() == 2
                && let (Ok(x), Ok(y)) = (parts[0].parse::<f32>(), parts[1].parse::<f32>())
            {
                return [x, y];
            }
        }
        [0.5, 0.5]
    }

    fn frame_anchors(
        avatar: &Avatar,
        animation_name: &str,
        direction: AvatarDirection,
        frame_index: usize,
    ) -> (Option<(i16, i16)>, Option<(i16, i16)>) {
        let Some(anim) = avatar
            .animations
            .iter()
            .find(|a| a.name.eq_ignore_ascii_case(animation_name))
            .or_else(|| avatar.animations.first())
        else {
            return (None, None);
        };
        let Some(perspective) = anim
            .perspectives
            .iter()
            .find(|p| p.direction == direction)
            .or_else(|| {
                anim.perspectives
                    .iter()
                    .find(|p| p.direction == AvatarDirection::Front)
            })
            .or_else(|| anim.perspectives.first())
        else {
            return (None, None);
        };
        let Some(frame) = perspective
            .frames
            .get(frame_index % perspective.frames.len().max(1))
        else {
            return (
                perspective.weapon_main_anchor,
                perspective.weapon_off_anchor,
            );
        };
        (
            frame.weapon_main_anchor.or(perspective.weapon_main_anchor),
            frame.weapon_off_anchor.or(perspective.weapon_off_anchor),
        )
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
        let mut frames: FxHashMap<(String, AvatarDirection, usize), (u32, Vec<u8>)> =
            FxHashMap::default();

        for anim in &avatar.animations {
            for perspective in &anim.perspectives {
                let frame_count = perspective.frames.len().max(1);
                for frame_index in 0..frame_count {
                    if let Some(out) = Self::build_preview_for_entity(
                        entity,
                        avatar,
                        assets,
                        Some(anim.name.as_str()),
                        perspective.direction,
                        frame_index,
                    ) {
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
