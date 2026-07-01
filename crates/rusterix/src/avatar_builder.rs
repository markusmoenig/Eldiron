use crate::{
    Assets, Avatar, AvatarBuildOutput, AvatarBuildRequest, AvatarBuilder, AvatarDirection,
    AvatarMarkerColors, AvatarShadingOptions, Entity, Item, PixelSource, Value,
};
use rustc_hash::{FxHashMap, FxHashSet};
use scenevm::{Atom, GeoId, SceneVM};
use std::hash::{Hash, Hasher};
use theframework::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AvatarFrameStyle {
    pub outline_color: [u8; 4],
    pub outline_thickness: usize,
}

struct CachedAvatarFrames {
    avatar_signature: u64,
    marker_signature: u64,
    frames: FxHashMap<(String, AvatarDirection, usize), (u32, Vec<u8>)>,
    scale_reference_heights: FxHashMap<AvatarDirection, f32>,
    last_uploaded: Option<(String, AvatarDirection, usize)>,
}

struct AvatarPlaybackState {
    animation_name: String,
    started_at_frame: usize,
}

#[derive(Default)]
pub struct AvatarRuntimeBuilder {
    // Latch for update_avatar=true edge detection.
    avatar_rebuild_latch: FxHashSet<GeoId>,
    // Per-entity built avatar frames (all anims/perspectives/frames).
    avatar_frame_cache: FxHashMap<GeoId, CachedAvatarFrames>,
    // Per-entity playback phase so animation switches (e.g. Attack) can start from frame 0.
    avatar_playback_state: FxHashMap<GeoId, AvatarPlaybackState>,
    shading_options: AvatarShadingOptions,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WeaponLayer {
    PreBody,
    Front,
    Back,
}

impl AvatarRuntimeBuilder {
    pub fn has_avatar_binding(entity: &Entity) -> bool {
        entity.attributes.get("avatar_id").is_some() || entity.attributes.get("avatar").is_some()
    }

    pub fn has_cached_avatar(&self, id: GeoId) -> bool {
        self.avatar_frame_cache.contains_key(&id)
    }

    pub(crate) fn avatar_definition_signature(avatar: &Avatar) -> u64 {
        let mut hasher = rustc_hash::FxHasher::default();
        avatar.id.hash(&mut hasher);
        avatar.name.hash(&mut hasher);
        avatar.resolution.hash(&mut hasher);
        avatar.perspective_count.hash(&mut hasher);
        avatar.animations.len().hash(&mut hasher);
        for anim in &avatar.animations {
            anim.id.hash(&mut hasher);
            anim.name.hash(&mut hasher);
            hasher.write_u32(anim.speed.to_bits());
            anim.perspectives.len().hash(&mut hasher);
            for perspective in &anim.perspectives {
                perspective.direction.hash(&mut hasher);
                perspective.weapon_main_anchor.hash(&mut hasher);
                perspective.weapon_off_anchor.hash(&mut hasher);
                perspective.frames.len().hash(&mut hasher);
                for frame in &perspective.frames {
                    frame.weapon_main_anchor.hash(&mut hasher);
                    frame.weapon_off_anchor.hash(&mut hasher);
                    frame.texture.width.hash(&mut hasher);
                    frame.texture.height.hash(&mut hasher);
                    frame.texture.data.hash(&mut hasher);
                }
            }
        }
        hasher.finish()
    }

    fn marker_colors_signature(colors: &AvatarMarkerColors) -> u64 {
        let mut hasher = rustc_hash::FxHasher::default();
        colors.skin_light.hash(&mut hasher);
        colors.skin_dark.hash(&mut hasher);
        colors.torso.hash(&mut hasher);
        colors.arms.hash(&mut hasher);
        colors.legs.hash(&mut hasher);
        colors.hair.hash(&mut hasher);
        colors.eyes.hash(&mut hasher);
        colors.hands.hash(&mut hasher);
        colors.feet.hash(&mut hasher);
        hasher.finish()
    }

    pub fn build_preview_for_entity(
        entity: &Entity,
        avatar: &Avatar,
        assets: &Assets,
        animation_name: Option<&str>,
        direction: AvatarDirection,
        frame_index: usize,
        shading: AvatarShadingOptions,
    ) -> Option<AvatarBuildOutput> {
        Self::build_preview_for_entity_with_weapons(
            entity,
            avatar,
            assets,
            animation_name,
            direction,
            frame_index,
            shading,
            true,
        )
    }

    pub fn build_preview_for_entity_with_weapons(
        entity: &Entity,
        avatar: &Avatar,
        assets: &Assets,
        animation_name: Option<&str>,
        direction: AvatarDirection,
        frame_index: usize,
        shading: AvatarShadingOptions,
        include_weapons: bool,
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
            shading,
        })?;
        if !include_weapons {
            return Some(out);
        }
        let (main_anchor, off_anchor) =
            Self::frame_anchors(avatar, resolved_anim, resolved_dir, frame_index);
        let hand_mask = Self::frame_hand_mask(
            avatar,
            resolved_anim,
            resolved_dir,
            frame_index,
            out.size as usize,
        );
        let size = out.size as usize;
        let mut pre_body_overlay = vec![0_u8; size * size * 4];
        let mut back_overlay = vec![0_u8; size * size * 4];
        let mut front_overlay = vec![0_u8; size * size * 4];
        Self::compose_weapon_overlay(
            &mut pre_body_overlay,
            &mut back_overlay,
            &mut front_overlay,
            size,
            entity,
            assets,
            resolved_dir,
            frame_index,
            main_anchor,
            off_anchor,
        );
        let composed = if resolved_dir.is_back_facing() {
            // Back view: weapons belong behind the character body.
            let mut c = vec![0_u8; size * size * 4];
            Self::alpha_blit_rgba(&mut c, size, size, &pre_body_overlay, size, size, 0, 0);
            Self::alpha_blit_rgba(&mut c, size, size, &back_overlay, size, size, 0, 0);
            Self::alpha_blit_rgba(&mut c, size, size, &front_overlay, size, size, 0, 0);
            Self::alpha_blit_rgba(&mut c, size, size, &out.rgba, size, size, 0, 0);
            c
        } else {
            // Front/side views: pre-body layer, body, hand-back layer, then hand-front layer.
            let mut c = pre_body_overlay;
            Self::alpha_blit_rgba(&mut c, size, size, &out.rgba, size, size, 0, 0);
            Self::alpha_blit_rgba(&mut c, size, size, &back_overlay, size, size, 0, 0);
            if let Some(mask) = hand_mask {
                // Restore hand pixels on top of back-layer weapons so "back" means behind hand.
                for (i, is_hand) in mask.iter().enumerate() {
                    if *is_hand {
                        let idx = i * 4;
                        c[idx..idx + 4].copy_from_slice(&out.rgba[idx..idx + 4]);
                    }
                }
            }
            Self::alpha_blit_rgba(&mut c, size, size, &front_overlay, size, size, 0, 0);
            c
        };
        out.rgba = composed;
        Some(out)
    }

    pub(crate) fn explicit_item_tile(item: &Item, assets: &Assets) -> Option<crate::Tile> {
        if let Some(tile_id) = Self::item_tile_id_for_direction(item, AvatarDirection::Front) {
            return assets.tiles.get(&tile_id).cloned();
        }
        Self::item_class_tile_source(item, assets)
            .and_then(|source| Self::tile_from_pixel_source(&source, assets))
    }

    pub(crate) fn explicit_item_tile_id(item: &Item, assets: &Assets) -> Option<Uuid> {
        Self::item_tile_id_for_direction(item, AvatarDirection::Front).or_else(|| {
            Self::item_class_tile_source(item, assets)
                .and_then(|source| source.render_tile_id(assets))
        })
    }

    pub(crate) fn item_has_explicit_tile(item: &Item, assets: &Assets) -> bool {
        Self::item_tile_id_for_direction(item, AvatarDirection::Front).is_some()
            || Self::item_class_tile_source(item, assets).is_some()
    }

    pub(crate) fn item_allows_generated_icon(item: &Item, assets: &Assets) -> bool {
        if Self::item_has_explicit_tile(item, assets) {
            return false;
        }
        if Self::item_has_project_item_class(item, assets)
            && item.attributes.get_str("ruleset_id").is_none()
        {
            return false;
        }
        true
    }

    pub(crate) fn generated_item_tile(item: &Item, assets: &Assets) -> Option<crate::Tile> {
        let slot = item.attributes.get_str("slot").unwrap_or("item");
        let (rgba, width, height, _) = Self::generated_rig_texture(item, assets, slot)?;
        Some(crate::Tile::from_texture(crate::Texture::new(
            rgba, width, height,
        )))
    }

    fn item_has_project_item_class(item: &Item, assets: &Assets) -> bool {
        let Some(class_name) = item
            .attributes
            .get_str("class_name")
            .or_else(|| (!item.item_type.trim().is_empty()).then_some(item.item_type.as_str()))
        else {
            return false;
        };
        let class_name = class_name.trim();
        assets.items.contains_key(class_name)
            || assets
                .items
                .keys()
                .any(|name| name.eq_ignore_ascii_case(class_name))
    }

    fn item_class_tile_source(item: &Item, assets: &Assets) -> Option<PixelSource> {
        let class_name = item
            .attributes
            .get_str("class_name")
            .or_else(|| (!item.item_type.trim().is_empty()).then_some(item.item_type.as_str()))?
            .trim();
        let (_, data) = assets.items.get(class_name).or_else(|| {
            assets
                .items
                .iter()
                .find(|(name, _)| name.eq_ignore_ascii_case(class_name))
                .map(|(_, value)| value)
        })?;
        let table = data.parse::<toml::Table>().ok()?;
        Self::tile_source_from_item_data_table(&table)
    }

    fn tile_from_pixel_source(source: &PixelSource, assets: &Assets) -> Option<crate::Tile> {
        match source {
            PixelSource::TileId(id) => assets.tiles.get(id).cloned(),
            PixelSource::MaterialId(id) => assets.materials.get(id).cloned(),
            _ => source.tile_from_tile_list(assets),
        }
    }

    fn tile_source_from_item_data_table(table: &toml::Table) -> Option<PixelSource> {
        for key in ["tile_id", "rig_tile_id", "tile"] {
            if let Some(source) = table.get(key).and_then(Self::tile_source_from_toml_value) {
                return Some(source);
            }
        }
        let attrs = table.get("attributes").and_then(toml::Value::as_table)?;
        for key in ["tile_id", "rig_tile_id", "tile"] {
            if let Some(source) = attrs.get(key).and_then(Self::tile_source_from_toml_value) {
                return Some(source);
            }
        }
        None
    }

    fn tile_source_from_toml_value(value: &toml::Value) -> Option<PixelSource> {
        if let Some(raw) = value.as_str() {
            let raw = raw.trim();
            if let Ok(id) = Uuid::parse_str(raw) {
                return Some(PixelSource::TileId(id));
            }
            if let Ok(index) = raw.parse::<u16>() {
                return Some(PixelSource::PaletteIndex(index));
            }
        }
        if let Some(index) = value.as_integer()
            && (0..=u16::MAX as i64).contains(&index)
        {
            return Some(PixelSource::PaletteIndex(index as u16));
        }
        None
    }

    fn find_avatar_by_name<'a>(name: &str, assets: &'a Assets) -> Option<&'a Avatar> {
        if name.trim().is_empty() {
            return None;
        }
        if let Some(avatar) = assets.avatars.get(name) {
            return Some(avatar);
        }
        for (key, avatar) in &assets.avatars {
            if key.eq_ignore_ascii_case(name) || avatar.name.eq_ignore_ascii_case(name) {
                return Some(avatar);
            }
        }
        None
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
            return Self::find_avatar_by_name(name, assets);
        }
        if entity.attributes.get("source").is_some() || entity.attributes.get("tile_id").is_some() {
            return None;
        }
        assets
            .default_avatar
            .as_deref()
            .and_then(|name| Self::find_avatar_by_name(name, assets))
    }

    fn avatar_direction_from_entity(entity: &Entity) -> AvatarDirection {
        let dir = entity.orientation;
        AvatarDirection::from_xy(dir.x, dir.y)
    }

    fn marker_color_from_index(assets: &Assets, idx: i32, default: [u8; 4]) -> [u8; 4] {
        if idx < 0 {
            return default;
        }
        let i = idx as usize;
        if i < assets.ruleset_palette.colors.len() {
            if let Some(col) = &assets.ruleset_palette[i] {
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
            // Base/default marker color for most slots is light_skin.
            skin_light: light_skin,
            // Keep a distinct dark skin default even when only light_skin is configured.
            skin_dark: Self::marker_color(entity, assets, "dark_skin", defaults.skin_dark),
            torso: Self::marker_color(entity, assets, "torso", light_skin),
            arms: Self::marker_color(entity, assets, "arms", light_skin),
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
            if let Some(c) = Self::marker_color_from_item(item, assets, "arms") {
                colors.arms = c;
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
        let directional_keys: Vec<String> = direction
            .fallback_directions()
            .iter()
            .flat_map(|dir| {
                let suffix = dir.key();
                [
                    format!("tile_id_{suffix}"),
                    format!("rig_tile_id_{suffix}"),
                    format!("tile_{suffix}"),
                ]
            })
            .collect();
        let generic_keys = ["tile_id", "rig_tile_id", "tile", "source"];

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

    fn has_directional_tile_for_direction(item: &Item, direction: AvatarDirection) -> bool {
        let directional_keys: Vec<String> = direction
            .fallback_directions()
            .iter()
            .flat_map(|dir| {
                let suffix = dir.key();
                [
                    format!("tile_id_{suffix}"),
                    format!("rig_tile_id_{suffix}"),
                    format!("tile_{suffix}"),
                ]
            })
            .collect();
        for key in directional_keys.iter().map(|s| s.as_str()) {
            if item.attributes.get_id(key).is_some() {
                return true;
            }
            if let Some(PixelSource::TileId(_)) = item.attributes.get_source(key) {
                return true;
            }
            if let Some(raw) = item.attributes.get_str(key)
                && Uuid::parse_str(raw).is_ok()
            {
                return true;
            }
        }
        false
    }

    fn weapon_order_for_direction(direction: AvatarDirection) -> [(&'static str, bool); 2] {
        if direction.is_back_facing() {
            [("main_hand", true), ("off_hand", false)]
        } else {
            [("off_hand", false), ("main_hand", true)]
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

    fn scaled_rgba(
        rgba: &[u8],
        width: usize,
        height: usize,
        scale: f32,
    ) -> Option<(Vec<u8>, usize, usize)> {
        if width == 0 || height == 0 || rgba.len() != width * height * 4 {
            return None;
        }
        let scale = if scale.is_finite() {
            scale.max(0.01)
        } else {
            1.0
        };
        let out_w = ((width as f32) * scale).round().max(1.0) as usize;
        let out_h = ((height as f32) * scale).round().max(1.0) as usize;
        let mut out = vec![0_u8; out_w * out_h * 4];
        for y in 0..out_h {
            let src_y = ((y as f32) / scale)
                .floor()
                .clamp(0.0, (height.saturating_sub(1)) as f32) as usize;
            for x in 0..out_w {
                let src_x = ((x as f32) / scale)
                    .floor()
                    .clamp(0.0, (width.saturating_sub(1)) as f32)
                    as usize;
                let src_i = (src_y * width + src_x) * 4;
                let dst_i = (y * out_w + x) * 4;
                out[dst_i..dst_i + 4].copy_from_slice(&rgba[src_i..src_i + 4]);
            }
        }
        Some((out, out_w, out_h))
    }

    fn item_visual_color(item: &Item, assets: &Assets, default: [u8; 4]) -> [u8; 4] {
        if let Some(Value::Color(c)) = item.attributes.get("rig_color") {
            return c.to_u8_array();
        }
        if let Some(Value::Color(c)) = item.attributes.get("color") {
            return c.to_u8_array();
        }
        if let Some(hex) = item.attributes.get_str("rig_color") {
            return TheColor::from_hex(hex).to_u8_array();
        }
        if let Some(hex) = item.attributes.get_str("color") {
            return TheColor::from_hex(hex).to_u8_array();
        }
        if let Some(idx) = item
            .attributes
            .get_int("rig_color")
            .or_else(|| item.attributes.get_int("color"))
            .or_else(|| item.attributes.get_int("rig_color_index"))
            .or_else(|| item.attributes.get_int("color_index"))
        {
            return Self::marker_color_from_index(assets, idx, default);
        }
        default
    }

    fn item_role_visual_color(
        item: &Item,
        assets: &Assets,
        role: &str,
        default: [u8; 4],
    ) -> [u8; 4] {
        let color_key = format!("{role}_color");
        let index_key = format!("{role}_color_index");
        if let Some(Value::Color(c)) = item.attributes.get(&color_key) {
            return c.to_u8_array();
        }
        if let Some(hex) = item.attributes.get_str(&color_key) {
            return TheColor::from_hex(hex).to_u8_array();
        }
        if let Some(idx) = item.attributes.get_int(&color_key) {
            return Self::marker_color_from_index(assets, idx, default);
        }
        if let Some(idx) = item.attributes.get_int(&index_key) {
            return Self::marker_color_from_index(assets, idx, default);
        }
        default
    }

    fn put_weapon_pixel(rgba: &mut [u8], width: usize, x: i32, y: i32, color: [u8; 4]) {
        if x < 0 || y < 0 {
            return;
        }
        let x = x as usize;
        let y = y as usize;
        if x >= width || y >= rgba.len() / (width * 4) {
            return;
        }
        let i = (y * width + x) * 4;
        rgba[i..i + 4].copy_from_slice(&color);
    }

    fn draw_weapon_rect(
        rgba: &mut [u8],
        width: usize,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        color: [u8; 4],
    ) {
        for yy in y..y + h {
            for xx in x..x + w {
                Self::put_weapon_pixel(rgba, width, xx, yy, color);
            }
        }
    }

    fn draw_weapon_line(
        rgba: &mut [u8],
        width: usize,
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        color: [u8; 4],
    ) {
        let mut x = x0;
        let mut y = y0;
        let dx = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let dy = -(y1 - y0).abs();
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        loop {
            Self::put_weapon_pixel(rgba, width, x, y, color);
            if x == x1 && y == y1 {
                break;
            }
            let e2 = err * 2;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }

    fn item_template_mask_rgba(
        item: &Item,
        blade: [u8; 4],
        grip: [u8; 4],
        accent: [u8; 4],
        highlight: [u8; 4],
    ) -> Option<(Vec<u8>, usize, usize)> {
        let width = item.attributes.get_int("visual_template_width")? as usize;
        let height = item.attributes.get_int("visual_template_height")? as usize;
        let Some(Value::StrArray(rows)) = item.attributes.get("visual_template_pixels") else {
            return None;
        };
        if width == 0 || height == 0 || rows.len() != height {
            return None;
        }

        let mut rgba = vec![0_u8; width * height * 4];
        for (y, row) in rows.iter().enumerate() {
            if row.chars().count() != width {
                return None;
            }
            for (x, ch) in row.chars().enumerate() {
                let color = match ch {
                    'B' | 'b' => blade,
                    'G' | 'g' => grip,
                    'A' | 'a' => accent,
                    'H' | 'h' => highlight,
                    '.' | ' ' => continue,
                    _ => continue,
                };
                let i = (y * width + x) * 4;
                rgba[i..i + 4].copy_from_slice(&color);
            }
        }

        Some((rgba, width, height))
    }

    fn generated_rig_texture(
        item: &Item,
        assets: &Assets,
        slot: &str,
    ) -> Option<(Vec<u8>, usize, usize, [f32; 2])> {
        let category = item
            .attributes
            .get_str("category")
            .or_else(|| item.attributes.get_str("ruleset_kind"))
            .unwrap_or(slot)
            .trim()
            .to_ascii_lowercase();
        let template = item
            .attributes
            .get_str("rig_template")
            .or_else(|| item.attributes.get_str("visual_template"))
            .or_else(|| item.attributes.get_str("icon_template"))
            .unwrap_or(&category)
            .trim()
            .to_ascii_lowercase();
        let metal = Self::item_role_visual_color(
            item,
            assets,
            "blade",
            Self::item_visual_color(item, assets, [187, 195, 208, 255]),
        );
        let wood = Self::item_role_visual_color(
            item,
            assets,
            "grip",
            Self::marker_color_from_index(assets, 10, [165, 120, 80, 255]),
        );
        let accent = Self::item_role_visual_color(item, assets, "accent", [48, 56, 67, 255]);
        let dark = [48, 56, 67, 255];
        let highlight =
            Self::item_role_visual_color(item, assets, "highlight", [241, 246, 240, 255]);

        if let Some((rgba, width, height)) =
            Self::item_template_mask_rgba(item, metal, wood, accent, highlight)
        {
            let pivot = match template.as_str() {
                "sword_diagonal" => [0.31, 0.82],
                "shield" => [0.5, 0.5],
                _ => [0.5, 0.82],
            };
            return Some((rgba, width, height, pivot));
        }

        let (width, height, pivot) = match template.as_str() {
            "sword_diagonal" => (16usize, 16usize, [0.31, 0.82]),
            "shield" => (16usize, 18usize, [0.5, 0.5]),
            _ => (16usize, 24usize, [0.5, 0.82]),
        };
        let mut rgba = vec![0_u8; width * height * 4];

        match template.as_str() {
            "sword_diagonal" => {
                Self::draw_weapon_line(&mut rgba, width, 3, 12, 11, 4, metal);
                Self::draw_weapon_line(&mut rgba, width, 4, 12, 12, 4, metal);
                Self::draw_weapon_line(&mut rgba, width, 8, 14, 12, 10, accent);
                Self::draw_weapon_rect(&mut rgba, width, 2, 13, 3, 2, wood);
                Self::put_weapon_pixel(&mut rgba, width, 13, 3, highlight);
            }
            "sword" => {
                Self::draw_weapon_rect(&mut rgba, width, 7, 2, 2, 14, metal);
                Self::draw_weapon_rect(&mut rgba, width, 8, 1, 1, 1, highlight);
                Self::draw_weapon_rect(&mut rgba, width, 6, 16, 4, 1, accent);
                Self::draw_weapon_rect(&mut rgba, width, 7, 17, 2, 5, wood);
                Self::put_weapon_pixel(&mut rgba, width, 7, 0, highlight);
            }
            "axe" => {
                Self::draw_weapon_rect(&mut rgba, width, 7, 5, 2, 16, wood);
                Self::draw_weapon_rect(&mut rgba, width, 5, 4, 6, 5, metal);
                Self::put_weapon_pixel(&mut rgba, width, 4, 6, metal);
                Self::put_weapon_pixel(&mut rgba, width, 11, 6, metal);
                Self::draw_weapon_rect(&mut rgba, width, 6, 9, 4, 1, dark);
            }
            "mace" => {
                Self::draw_weapon_rect(&mut rgba, width, 7, 7, 2, 14, wood);
                Self::draw_weapon_rect(&mut rgba, width, 5, 3, 6, 5, metal);
                Self::put_weapon_pixel(&mut rgba, width, 4, 5, metal);
                Self::put_weapon_pixel(&mut rgba, width, 11, 5, metal);
                Self::draw_weapon_rect(&mut rgba, width, 6, 8, 4, 1, dark);
            }
            "shield" => {
                Self::draw_weapon_rect(&mut rgba, width, 4, 3, 8, 11, metal);
                Self::draw_weapon_rect(&mut rgba, width, 5, 2, 6, 13, metal);
                Self::draw_weapon_rect(&mut rgba, width, 6, 4, 4, 9, wood);
                Self::draw_weapon_rect(&mut rgba, width, 7, 3, 2, 11, highlight);
                Self::put_weapon_pixel(&mut rgba, width, 7, 15, metal);
                Self::put_weapon_pixel(&mut rgba, width, 8, 15, metal);
            }
            "bow" => {
                for y in 2..22 {
                    let x = if y < 8 {
                        6
                    } else if y < 16 {
                        5
                    } else {
                        6
                    };
                    Self::put_weapon_pixel(&mut rgba, width, x, y, wood);
                }
                Self::draw_weapon_rect(&mut rgba, width, 10, 3, 1, 18, highlight);
            }
            _ => return None,
        }

        Some((rgba, width, height, pivot))
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

    fn item_rig_layer(item: &Item, direction: AvatarDirection, slot: &str) -> WeaponLayer {
        let default_layer = if direction.is_back_facing() {
            WeaponLayer::PreBody
        } else {
            match direction {
                // Side depth hint:
                // Right view => off-hand behind body, main-hand in front.
                AvatarDirection::Right | AvatarDirection::FrontRight if slot == "off_hand" => {
                    WeaponLayer::PreBody
                }
                _ => WeaponLayer::Front,
            }
        };
        let Some(raw) = item.attributes.get_str("rig_layer") else {
            return default_layer;
        };
        match raw.to_ascii_lowercase().as_str() {
            "back" | "behind" | "under" => WeaponLayer::Back,
            "pre_body" | "body_back" | "behind_body" => WeaponLayer::PreBody,
            "front" | "over" => WeaponLayer::Front,
            _ => default_layer,
        }
    }

    fn frame_hand_mask(
        avatar: &Avatar,
        animation_name: &str,
        direction: AvatarDirection,
        frame_index: usize,
        target_size: usize,
    ) -> Option<Vec<bool>> {
        const HANDS: [u8; 3] = [255, 128, 0];
        let anim = avatar
            .animations
            .iter()
            .find(|a| a.name.eq_ignore_ascii_case(animation_name))
            .or_else(|| avatar.animations.first())?;
        let perspective = anim
            .perspectives
            .iter()
            .find(|p| p.direction == direction)
            .or_else(|| {
                direction
                    .fallback_directions()
                    .iter()
                    .filter(|dir| **dir != direction)
                    .find_map(|dir| anim.perspectives.iter().find(|p| p.direction == *dir))
            })
            .or_else(|| anim.perspectives.first())?;
        if perspective.frames.is_empty() {
            return None;
        }
        let frame = perspective
            .frames
            .get(frame_index % perspective.frames.len())
            .or_else(|| perspective.frames.first())?;
        let tex = if frame.texture.width == frame.texture.height {
            frame.texture.clone()
        } else {
            frame.texture.resized(
                frame.texture.width.max(frame.texture.height),
                frame.texture.width.max(frame.texture.height),
            )
        };
        if tex.width == 0 || tex.height == 0 {
            return None;
        }
        let mut mask = vec![false; target_size * target_size];
        let sx = tex.width as f32 / target_size as f32;
        let sy = tex.height as f32 / target_size as f32;
        for y in 0..target_size {
            let src_y = ((y as f32 * sy).floor() as usize).min(tex.height - 1);
            for x in 0..target_size {
                let src_x = ((x as f32 * sx).floor() as usize).min(tex.width - 1);
                let src_i = (src_y * tex.width + src_x) * 4;
                let alpha = tex.data[src_i + 3];
                if alpha == 0 {
                    continue;
                }
                if tex.data[src_i] == HANDS[0]
                    && tex.data[src_i + 1] == HANDS[1]
                    && tex.data[src_i + 2] == HANDS[2]
                {
                    mask[y * target_size + x] = true;
                }
            }
        }
        Some(mask)
    }

    fn compose_weapon_overlay(
        pre_body_overlay: &mut [u8],
        back_overlay: &mut [u8],
        front_overlay: &mut [u8],
        out_size: usize,
        entity: &Entity,
        assets: &Assets,
        direction: AvatarDirection,
        frame_index: usize,
        main_anchor: Option<(i16, i16)>,
        off_anchor: Option<(i16, i16)>,
    ) {
        let preview_debug = entity
            .attributes
            .get_bool_default("avatar_preview_debug", false);
        for (slot, is_main) in Self::weapon_order_for_direction(direction) {
            let anchor = if is_main { main_anchor } else { off_anchor };
            let Some(anchor) = anchor else {
                if preview_debug {
                    eprintln!(
                        "[RIGPREVIEW] overlay slot='{}' -> no anchor (main={:?} off={:?})",
                        slot, main_anchor, off_anchor
                    );
                }
                continue;
            };
            let Some(item) = Self::find_equipped_for_slot(entity, slot) else {
                if preview_debug {
                    eprintln!(
                        "[RIGPREVIEW] overlay slot='{}' -> no equipped item for slot aliases",
                        slot
                    );
                }
                continue;
            };
            let scale = item.attributes.get_float_default("rig_scale", 1.0);
            let has_directional_tile;
            let debug_source;
            let (mut scaled, sw, sh, mut pivot) = if let Some(tile_id) =
                Self::item_tile_id_for_direction(item, direction)
            {
                let Some(tile) = assets.tiles.get(&tile_id) else {
                    if preview_debug {
                        eprintln!(
                            "[RIGPREVIEW] overlay slot='{}' -> tile '{}' not found in assets",
                            slot, tile_id
                        );
                    }
                    continue;
                };
                if tile.textures.is_empty() {
                    if preview_debug {
                        eprintln!(
                            "[RIGPREVIEW] overlay slot='{}' -> tile '{}' has no textures",
                            slot, tile_id
                        );
                    }
                    continue;
                }
                let tex = &tile.textures[frame_index % tile.textures.len()];
                let Some((scaled, sw, sh)) = Self::scaled_texture_rgba(tex, scale) else {
                    if preview_debug {
                        eprintln!(
                            "[RIGPREVIEW] overlay slot='{}' -> texture scale failed (w={} h={} scale={})",
                            slot, tex.width, tex.height, scale
                        );
                    }
                    continue;
                };
                has_directional_tile = Self::has_directional_tile_for_direction(item, direction);
                debug_source = tile_id.to_string();
                (scaled, sw, sh, Self::item_rig_pivot(item))
            } else if let Some((rgba, width, height, default_pivot)) =
                Self::generated_rig_texture(item, assets, slot)
            {
                let Some((scaled, sw, sh)) = Self::scaled_rgba(&rgba, width, height, scale) else {
                    continue;
                };
                has_directional_tile = false;
                debug_source = "generated".to_string();
                (scaled, sw, sh, Self::item_rig_pivot_or(item, default_pivot))
            } else {
                if preview_debug {
                    eprintln!(
                        "[RIGPREVIEW] overlay slot='{}' -> no tile_id or generated rig visual for direction {:?}",
                        slot, direction
                    );
                }
                continue;
            };
            let flip_back = item.attributes.get_bool_default("rig_flip_back", true);
            let should_flip = direction.is_left_facing()
                || (direction.is_back_facing() && !has_directional_tile && flip_back);
            if should_flip {
                pivot[0] = 1.0 - pivot[0];
                scaled = Self::flip_rgba_horizontal(&scaled, sw, sh);
            }
            let px = (pivot[0].clamp(0.0, 1.0) * (sw as f32 - 1.0)).round() as i32;
            let py = (pivot[1].clamp(0.0, 1.0) * (sh as f32 - 1.0)).round() as i32;
            let dst_x = anchor.0 as i32 - px;
            let dst_y = anchor.1 as i32 - py;
            let layer = Self::item_rig_layer(item, direction, slot);
            if preview_debug {
                eprintln!(
                    "[RIGPREVIEW] overlay slot='{}' layer={:?} anchor=({}, {}) tile='{}' tex={}x{} scale={} flip={} pivot=({:.2},{:.2}) dst=({}, {})",
                    slot,
                    layer,
                    anchor.0,
                    anchor.1,
                    debug_source,
                    sw,
                    sh,
                    scale,
                    should_flip,
                    pivot[0],
                    pivot[1],
                    dst_x,
                    dst_y
                );
            }
            match layer {
                WeaponLayer::PreBody => Self::alpha_blit_rgba(
                    pre_body_overlay,
                    out_size,
                    out_size,
                    &scaled,
                    sw,
                    sh,
                    dst_x,
                    dst_y,
                ),
                WeaponLayer::Back => Self::alpha_blit_rgba(
                    back_overlay,
                    out_size,
                    out_size,
                    &scaled,
                    sw,
                    sh,
                    dst_x,
                    dst_y,
                ),
                WeaponLayer::Front => Self::alpha_blit_rgba(
                    front_overlay,
                    out_size,
                    out_size,
                    &scaled,
                    sw,
                    sh,
                    dst_x,
                    dst_y,
                ),
            }
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

    fn item_rig_pivot_or(item: &Item, default: [f32; 2]) -> [f32; 2] {
        if item.attributes.get("rig_pivot").is_some() {
            Self::item_rig_pivot(item)
        } else {
            default
        }
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
                direction
                    .fallback_directions()
                    .iter()
                    .filter(|dir| **dir != direction)
                    .find_map(|dir| anim.perspectives.iter().find(|p| p.direction == *dir))
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
                direction
                    .fallback_directions()
                    .iter()
                    .filter(|dir| **dir != direction)
                    .find_map(|dir| anim.perspectives.iter().find(|p| p.direction == *dir))
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
        avatar_signature: u64,
        marker_signature: u64,
    ) -> bool {
        let mut frames: FxHashMap<(String, AvatarDirection, usize), (u32, Vec<u8>)> =
            FxHashMap::default();
        let mut scale_reference_heights: FxHashMap<AvatarDirection, f32> = FxHashMap::default();

        for anim in &avatar.animations {
            for perspective in &anim.perspectives {
                let frame_count = perspective.frames.len().max(1);
                for frame_index in 0..frame_count {
                    if anim.name.eq_ignore_ascii_case("idle") {
                        if let Some(out) = Self::build_preview_for_entity_with_weapons(
                            entity,
                            avatar,
                            assets,
                            Some(anim.name.as_str()),
                            perspective.direction,
                            frame_index,
                            self.shading_options,
                            false,
                        ) {
                            if let Some((_, top, _, bottom)) =
                                Self::alpha_bounds(out.size, &out.rgba)
                            {
                                scale_reference_heights
                                    .entry(perspective.direction)
                                    .and_modify(|height| *height = height.max(bottom - top))
                                    .or_insert(bottom - top);
                            }
                        }
                    }

                    if let Some(out) = Self::build_preview_for_entity(
                        entity,
                        avatar,
                        assets,
                        Some(anim.name.as_str()),
                        perspective.direction,
                        frame_index,
                        self.shading_options,
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
                avatar_signature,
                marker_signature,
                frames,
                scale_reference_heights,
                last_uploaded: None,
            },
        );
        // println!("rebuilt avatar entity {} cache", entity.id);
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
        self.ensure_entity_avatar_uploaded_with_direction(
            vm,
            entity,
            avatar,
            assets,
            frame_index,
            geo_id,
            None,
        )
    }

    pub fn ensure_entity_avatar_uploaded_with_direction(
        &mut self,
        vm: &mut SceneVM,
        entity: &Entity,
        avatar: &Avatar,
        assets: &Assets,
        frame_index: usize,
        geo_id: GeoId,
        direction_override: Option<AvatarDirection>,
    ) -> bool {
        self.ensure_entity_avatar_uploaded_with_direction_and_style(
            vm,
            entity,
            avatar,
            assets,
            frame_index,
            geo_id,
            direction_override,
            None,
        )
    }

    pub fn ensure_entity_avatar_uploaded_with_direction_and_style(
        &mut self,
        vm: &mut SceneVM,
        entity: &Entity,
        avatar: &Avatar,
        assets: &Assets,
        frame_index: usize,
        geo_id: GeoId,
        direction_override: Option<AvatarDirection>,
        frame_style: Option<AvatarFrameStyle>,
    ) -> bool {
        let update_avatar = entity.attributes.get_bool_default("update_avatar", false);
        let needs_rebuild_edge = if update_avatar {
            self.avatar_rebuild_latch.insert(geo_id)
        } else {
            self.avatar_rebuild_latch.remove(&geo_id);
            false
        };
        let avatar_signature = Self::avatar_definition_signature(avatar);
        let marker_signature =
            Self::marker_colors_signature(&Self::marker_colors_for_entity(entity, assets));
        let cache_missing = !self.avatar_frame_cache.contains_key(&geo_id);
        let cache_stale = self.avatar_frame_cache.get(&geo_id).is_some_and(|cache| {
            cache.avatar_signature != avatar_signature || cache.marker_signature != marker_signature
        });

        if (needs_rebuild_edge || cache_missing || cache_stale)
            && !self.rebuild_entity_avatar_cache(
                vm,
                entity,
                avatar,
                assets,
                geo_id,
                avatar_signature,
                marker_signature,
            )
        {
            return false;
        }

        let direction =
            direction_override.unwrap_or_else(|| Self::avatar_direction_from_entity(entity));
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

        let started_at = {
            let state =
                self.avatar_playback_state
                    .entry(geo_id)
                    .or_insert_with(|| AvatarPlaybackState {
                        animation_name: anim_name.to_string(),
                        started_at_frame: frame_index,
                    });
            if !state.animation_name.eq_ignore_ascii_case(anim_name) {
                state.animation_name = anim_name.to_string();
                state.started_at_frame = frame_index;
            }
            state.started_at_frame
        };

        // Apply per-animation playback speed:
        // speed = 1.0 normal, >1.0 slower, <1.0 faster.
        let speed = anim.speed.max(0.01);
        let local_frame = frame_index.saturating_sub(started_at);
        let scaled_frame = (local_frame as f32 / speed).floor() as usize;
        let frame_idx = if anim_name.eq_ignore_ascii_case("attack") {
            scaled_frame.min(frame_count.saturating_sub(1))
        } else {
            scaled_frame % frame_count
        };
        let key = (anim_name.to_string(), persp_dir, frame_idx);

        let Some(cache) = self.avatar_frame_cache.get_mut(&geo_id) else {
            return false;
        };
        let fallback_key = cache.last_uploaded.as_ref().and_then(|last| {
            if last.0.eq_ignore_ascii_case(anim_name) && last.1 == persp_dir {
                Some(last.clone())
            } else {
                None
            }
        });
        let Some((resolved_key, size, mut rgba)) = cache
            .frames
            .get(&key)
            .map(|entry| (key.clone(), entry.0, entry.1.clone()))
            .or_else(|| {
                fallback_key.as_ref().and_then(|last| {
                    cache
                        .frames
                        .get(last)
                        .map(|entry| (last.clone(), entry.0, entry.1.clone()))
                })
            })
            .or_else(|| {
                let zero_key = (anim_name.to_string(), persp_dir, 0);
                cache
                    .frames
                    .get(&zero_key)
                    .map(|entry| (zero_key, entry.0, entry.1.clone()))
            })
            .or_else(|| {
                cache.frames.iter().find_map(|(candidate, entry)| {
                    if candidate.0.eq_ignore_ascii_case(anim_name) && candidate.1 == persp_dir {
                        Some((candidate.clone(), entry.0, entry.1.clone()))
                    } else {
                        None
                    }
                })
            })
        else {
            return false;
        };

        if let Some(style) = frame_style
            && style.outline_color[3] > 0
            && style.outline_thickness > 0
        {
            Self::draw_alpha_outline(
                size as usize,
                &mut rgba,
                style.outline_color,
                style.outline_thickness,
            );
        }

        // Avatars are highly dynamic (animation + gear swaps), so always push the current frame,
        // including fully transparent frames.
        vm.execute(Atom::SetAvatarBillboardData {
            id: geo_id,
            size,
            rgba,
        });
        cache.last_uploaded = Some(resolved_key);
        true
    }

    fn draw_alpha_outline(size: usize, rgba: &mut [u8], color: [u8; 4], thickness: usize) {
        if size == 0 || rgba.len() < size * size * 4 || color[3] == 0 || thickness == 0 {
            return;
        }

        let source_alpha: Vec<u8> = rgba.chunks_exact(4).map(|px| px[3]).collect();
        let mut outline_alpha = vec![0_u8; size * size];
        let radius = thickness as isize;

        for y in 0..size {
            for x in 0..size {
                let idx = y * size + x;
                if source_alpha[idx] != 0 {
                    continue;
                }

                let x = x as isize;
                let y = y as isize;
                let mut near_body = false;
                'search: for oy in -radius..=radius {
                    for ox in -radius..=radius {
                        if ox == 0 && oy == 0 {
                            continue;
                        }
                        if ox.abs().max(oy.abs()) > radius {
                            continue;
                        }
                        let nx = x + ox;
                        let ny = y + oy;
                        if nx < 0 || ny < 0 || nx >= size as isize || ny >= size as isize {
                            continue;
                        }
                        if source_alpha[ny as usize * size + nx as usize] != 0 {
                            near_body = true;
                            break 'search;
                        }
                    }
                }

                if near_body {
                    outline_alpha[idx] = color[3];
                }
            }
        }

        for (idx, alpha) in outline_alpha.into_iter().enumerate() {
            if alpha == 0 {
                continue;
            }
            let offset = idx * 4;
            rgba[offset] = color[0];
            rgba[offset + 1] = color[1];
            rgba[offset + 2] = color[2];
            rgba[offset + 3] = alpha;
        }
    }

    pub fn current_avatar_alpha_bounds(&self, geo_id: GeoId) -> Option<(f32, f32, f32, f32, f32)> {
        let cache = self.avatar_frame_cache.get(&geo_id)?;
        let key = cache.last_uploaded.as_ref()?;
        let (size, rgba) = cache.frames.get(key)?;
        let bounds = Self::alpha_bounds(*size, rgba)?;
        let reference_h = cache
            .scale_reference_heights
            .get(&key.1)
            .copied()
            .unwrap_or_else(|| {
                cache
                    .frames
                    .iter()
                    .filter(|(candidate, _)| {
                        candidate.0.eq_ignore_ascii_case(&key.0) && candidate.1 == key.1
                    })
                    .filter_map(|(_, (frame_size, frame_rgba))| {
                        Self::alpha_bounds(*frame_size, frame_rgba)
                    })
                    .map(|(_, top, _, bottom)| bottom - top)
                    .fold(bounds.3 - bounds.1, f32::max)
            });
        Some((bounds.0, bounds.1, bounds.2, bounds.3, reference_h))
    }

    fn alpha_bounds(size: u32, rgba: &[u8]) -> Option<(f32, f32, f32, f32)> {
        let size = size as usize;
        if size == 0 || rgba.len() < size * size * 4 {
            return None;
        }

        let mut min_x = size;
        let mut min_y = size;
        let mut max_x = 0;
        let mut max_y = 0;
        let mut found = false;

        for y in 0..size {
            for x in 0..size {
                let alpha = rgba[(y * size + x) * 4 + 3];
                if alpha != 0 {
                    found = true;
                    min_x = min_x.min(x);
                    min_y = min_y.min(y);
                    max_x = max_x.max(x);
                    max_y = max_y.max(y);
                }
            }
        }

        if !found {
            return None;
        }

        let inv_size = 1.0 / size as f32;
        Some((
            min_x as f32 * inv_size,
            min_y as f32 * inv_size,
            (max_x + 1) as f32 * inv_size,
            (max_y + 1) as f32 * inv_size,
        ))
    }

    pub fn set_shading_options(&mut self, shading_options: AvatarShadingOptions) {
        if self.shading_options == shading_options {
            return;
        }
        self.shading_options = shading_options;
        // Shading affects generated frame pixels, so force full rebuild.
        self.avatar_frame_cache.clear();
        self.avatar_rebuild_latch.clear();
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
            self.avatar_playback_state.remove(&id);
            vm.execute(Atom::RemoveAvatarBillboardData { id });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AvatarAnimation, AvatarAnimationFrame, AvatarPerspective, AvatarPerspectiveCount, Texture,
    };

    fn solid_frame(rgba: [u8; 4]) -> AvatarAnimationFrame {
        AvatarAnimationFrame::new(Texture::new(rgba.to_vec(), 1, 1))
    }

    fn test_avatar_4dir() -> Avatar {
        Avatar {
            id: Uuid::new_v4(),
            name: "Human".to_string(),
            resolution: 1,
            perspective_count: AvatarPerspectiveCount::Four,
            animations: vec![AvatarAnimation {
                id: Uuid::new_v4(),
                name: "Idle".to_string(),
                speed: 1.0,
                perspectives: vec![
                    AvatarPerspective {
                        direction: AvatarDirection::Front,
                        frames: vec![solid_frame([255, 0, 0, 255])],
                        weapon_main_anchor: None,
                        weapon_off_anchor: None,
                    },
                    AvatarPerspective {
                        direction: AvatarDirection::Back,
                        frames: vec![solid_frame([0, 255, 0, 255])],
                        weapon_main_anchor: None,
                        weapon_off_anchor: None,
                    },
                    AvatarPerspective {
                        direction: AvatarDirection::Left,
                        frames: vec![solid_frame([0, 0, 255, 255])],
                        weapon_main_anchor: None,
                        weapon_off_anchor: None,
                    },
                    AvatarPerspective {
                        direction: AvatarDirection::Right,
                        frames: vec![solid_frame([255, 255, 0, 255])],
                        weapon_main_anchor: None,
                        weapon_off_anchor: None,
                    },
                ],
            }],
        }
    }

    #[test]
    fn avatar_cache_rebuilds_when_perspectives_change_to_eight() {
        let mut builder = AvatarRuntimeBuilder::default();
        let mut vm = SceneVM::default();
        let entity = Entity::new();
        let assets = Assets::default();
        let geo_id = GeoId::Character(7);
        let mut avatar = test_avatar_4dir();

        assert!(builder.ensure_entity_avatar_uploaded_with_direction(
            &mut vm,
            &entity,
            &avatar,
            &assets,
            0,
            geo_id,
            Some(AvatarDirection::Right),
        ));
        let old_signature = builder
            .avatar_frame_cache
            .get(&geo_id)
            .map(|cache| cache.avatar_signature)
            .unwrap();

        avatar.set_perspective_count(AvatarPerspectiveCount::Eight);
        assert!(builder.ensure_entity_avatar_uploaded_with_direction(
            &mut vm,
            &entity,
            &avatar,
            &assets,
            0,
            geo_id,
            Some(AvatarDirection::FrontRight),
        ));

        let cache = builder.avatar_frame_cache.get(&geo_id).unwrap();
        assert_ne!(cache.avatar_signature, old_signature);
        assert_eq!(
            cache.last_uploaded,
            Some(("Idle".to_string(), AvatarDirection::FrontRight, 0))
        );
        let (_, rgba) = cache
            .frames
            .get(&("Idle".to_string(), AvatarDirection::FrontRight, 0))
            .unwrap();
        assert_eq!(rgba, &[0, 0, 0, 0]);
    }
}
