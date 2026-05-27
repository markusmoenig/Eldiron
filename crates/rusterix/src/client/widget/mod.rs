pub mod avatar;
pub mod deco;
pub mod game;
pub mod game_backend;
pub mod messages;
pub mod screen;
pub mod text;

use crate::{
    Assets, Entity, Item, Map, Pixel, PlayerCamera, Rect, Texture, Value, WHITE,
    client::command::{ClientCommandBinding, parse_client_command},
    client::draw2d,
};
use draw2d::Draw2D;
use theframework::prelude::*;

/// Used right now for button widgets
pub struct Widget {
    pub name: String,
    pub id: u32,
    pub rect: Rect,
    pub action: String,
    pub command: Option<String>,
    pub intent: Option<String>,
    pub spell: Option<String>,
    pub group: Option<String>,
    pub show: Option<Vec<String>>,
    pub hide: Option<Vec<String>>,
    pub deactivate: Vec<String>,
    pub camera: Option<PlayerCamera>,
    pub player_camera: Option<PlayerCamera>,
    pub camera_target: Option<String>,
    pub party: Option<String>,
    pub inventory_index: Option<usize>,
    pub equipped_slot: Option<String>,
    pub portrait: bool,
    pub drag_drop: bool,
    pub textures: Vec<Texture>,
    pub entity_cursor_id: Option<Uuid>,
    pub entity_clicked_cursor_id: Option<Uuid>,
    pub item_cursor_id: Option<Uuid>,
    pub item_clicked_cursor_id: Option<Uuid>,
    pub border_color: Pixel,
    pub border_size: i32,
}

impl Default for Widget {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget {
    pub fn new() -> Self {
        Self {
            name: String::new(),
            id: 0,
            rect: Rect::default(),
            action: String::new(),
            command: None,
            intent: None,
            spell: None,
            group: None,
            show: None,
            hide: None,
            deactivate: vec![],
            camera: None,
            player_camera: None,
            camera_target: None,
            party: None,
            inventory_index: None,
            equipped_slot: None,
            portrait: false,
            drag_drop: false,
            textures: vec![],
            entity_cursor_id: None,
            entity_clicked_cursor_id: None,
            item_cursor_id: None,
            item_clicked_cursor_id: None,
            border_color: WHITE,
            border_size: 0,
        }
    }

    pub fn command_binding(&self) -> Option<ClientCommandBinding> {
        self.command
            .as_deref()
            .and_then(parse_client_command)
            .or_else(|| {
                self.intent.as_ref().and_then(|intent| {
                    let intent = intent.trim();
                    if intent.is_empty() {
                        None
                    } else if intent.eq_ignore_ascii_case("spell") {
                        self.spell
                            .as_ref()
                            .map(|spell| spell.trim())
                            .filter(|spell| !spell.is_empty())
                            .map(|spell| ClientCommandBinding::Intent(format!("spell:{}", spell)))
                            .or_else(|| Some(ClientCommandBinding::Intent(intent.to_string())))
                    } else {
                        Some(ClientCommandBinding::Intent(intent.to_string()))
                    }
                })
            })
            .or_else(|| {
                self.action
                    .trim()
                    .parse::<crate::EntityAction>()
                    .ok()
                    .map(ClientCommandBinding::Control)
            })
    }

    pub fn intent_payload(&self) -> Option<String> {
        self.command_binding()
            .and_then(|binding| binding.intent_payload())
    }

    pub fn update_draw(
        &mut self,
        buffer: &mut TheRGBABuffer,
        _map: &Map,
        assets: &Assets,
        entity: Option<&Entity>,
        draw2d: &Draw2D,
        animation_frame: &usize,
        texture_index: usize,
    ) {
        let stride = buffer.stride();

        if !self.textures.is_empty() {
            draw2d.blend_scale_chunk(
                buffer.pixels_mut(),
                &(
                    self.rect.x as usize,
                    self.rect.y as usize,
                    self.rect.width as usize,
                    self.rect.height as usize,
                ),
                stride,
                &self.textures[texture_index].data,
                &(
                    self.textures[texture_index].width as usize,
                    self.textures[texture_index].height as usize,
                ),
            );
        }

        let entity = entity;
        let item_to_draw = if let Some(inventory_index) = &self.inventory_index {
            entity.and_then(|entity| {
                entity
                    .inventory
                    .get(*inventory_index)
                    .and_then(|item| item.as_ref())
            })
        } else if let Some(slot) = &self.equipped_slot {
            entity.and_then(|entity| entity.get_equipped_item(slot))
        } else {
            None
        };

        if self.portrait
            && let Some(entity) = entity
            && let Some(tile) = Self::portrait_tile_for_entity(entity, assets)
        {
            let index = *animation_frame % tile.textures.len();
            let rect = self.rect.with_border(4.0);
            draw2d.blend_scale_chunk(
                buffer.pixels_mut(),
                &(
                    rect.x as usize,
                    rect.y as usize,
                    rect.width as usize,
                    rect.height as usize,
                ),
                stride,
                &tile.textures[index].data,
                &(
                    tile.textures[index].width as usize,
                    tile.textures[index].height as usize,
                ),
            );
        } else if let Some(item) = item_to_draw {
            Self::draw_item_icon(buffer, self.rect, assets, item, draw2d, *animation_frame);
        }

        if self.border_size > 0 {
            draw2d.rect_outline_thickness(
                buffer.pixels_mut(),
                &(
                    self.rect.x as usize,
                    self.rect.y as usize,
                    self.rect.width as usize,
                    self.rect.height as usize,
                ),
                stride,
                &self.border_color,
                self.border_size as usize,
            );
        }
    }

    fn portrait_tile_for_entity(entity: &Entity, assets: &Assets) -> Option<crate::Tile> {
        if let Some(source) = entity.attributes.get_source("portrait_tile_id") {
            return source.tile_from_tile_list(assets);
        }
        if let Some(id) = entity.attributes.get_id("portrait_tile_id") {
            return assets.tiles.get(&id).cloned();
        }
        entity
            .attributes
            .get_str("portrait_tile_id")
            .and_then(|value| Uuid::parse_str(value.trim()).ok())
            .and_then(|id| assets.tiles.get(&id).cloned())
    }

    pub fn draw_item_icon(
        buffer: &mut TheRGBABuffer,
        rect: Rect,
        assets: &Assets,
        item: &Item,
        draw2d: &Draw2D,
        animation_frame: usize,
    ) -> bool {
        let stride = buffer.stride();
        let rect = rect.with_border(4.0);
        let mut drawn = false;
        if let Some(Value::Source(source)) = item.attributes.get("source")
            && let Some(tile) = source.tile_from_tile_list(assets)
        {
            let index = animation_frame % tile.textures.len();
            let texture = &tile.textures[index];
            draw2d.blend_scale_chunk(
                buffer.pixels_mut(),
                &(
                    rect.x as usize,
                    rect.y as usize,
                    rect.width as usize,
                    rect.height as usize,
                ),
                stride,
                &texture.data,
                &(texture.width, texture.height),
            );
            drawn = true;
        }

        if !drawn && Self::draw_generated_avatar_channel_icon(buffer, rect, assets, item, draw2d) {
            drawn = true;
        }

        if !drawn {
            drawn = Self::draw_generated_equipment_icon(buffer, rect, assets, item, draw2d);
        }
        if drawn {
            Self::draw_stack_badge(buffer, rect, item, draw2d);
        }
        drawn
    }

    fn draw_stack_badge(buffer: &mut TheRGBABuffer, rect: Rect, item: &Item, _draw2d: &Draw2D) {
        let quantity = item.stack_quantity();
        if quantity <= 1 {
            return;
        }
        let text = quantity.min(999).to_string();
        let digit_w = 6_i32;
        let digit_h = 10_i32;
        let spacing = 2_i32;
        let text_w = text.len() as i32 * digit_w + (text.len().saturating_sub(1) as i32 * spacing);
        let x = (rect.x + rect.width - text_w as f32 - 2.0).round() as i32;
        let y = (rect.y + rect.height - digit_h as f32 - 2.0).round() as i32;
        let mut cursor_x = x;
        for ch in text.chars() {
            Self::draw_stack_digit(buffer, cursor_x + 1, y + 1, ch, &[18, 18, 18, 210]);
            Self::draw_stack_digit(buffer, cursor_x, y, ch, &[174, 179, 183, 255]);
            cursor_x += digit_w + spacing;
        }
    }

    fn draw_stack_digit(buffer: &mut TheRGBABuffer, x: i32, y: i32, ch: char, color: &[u8; 4]) {
        let pattern = match ch {
            '0' => ["111", "101", "101", "101", "111"],
            '1' => ["010", "110", "010", "010", "111"],
            '2' => ["111", "001", "111", "100", "111"],
            '3' => ["111", "001", "111", "001", "111"],
            '4' => ["101", "101", "111", "001", "001"],
            '5' => ["111", "100", "111", "001", "111"],
            '6' => ["111", "100", "111", "101", "111"],
            '7' => ["111", "001", "010", "010", "010"],
            '8' => ["111", "101", "111", "101", "111"],
            '9' => ["111", "101", "111", "001", "111"],
            _ => return,
        };
        let stride = buffer.stride();
        let pixels = buffer.pixels_mut();
        for (py, row) in pattern.iter().enumerate() {
            for (px, bit) in row.chars().enumerate() {
                if bit != '1' {
                    continue;
                }
                for oy in 0..2 {
                    for ox in 0..2 {
                        let sx = x + px as i32 * 2 + ox;
                        let sy = y + py as i32 * 2 + oy;
                        if sx < 0 || sy < 0 {
                            continue;
                        }
                        let index = (sy as usize * stride + sx as usize) * 4;
                        if index + 3 >= pixels.len() {
                            continue;
                        }
                        pixels[index..index + 4].copy_from_slice(color);
                    }
                }
            }
        }
    }

    pub(crate) fn item_generated_icon_square(
        assets: &Assets,
        item: &Item,
    ) -> Option<(u32, Vec<u8>)> {
        Self::item_avatar_channel_icon_square(assets, item)
            .or_else(|| Self::item_equipment_icon_square(assets, item))
    }

    pub(crate) fn item_avatar_channel_icon_square(
        assets: &Assets,
        item: &Item,
    ) -> Option<(u32, Vec<u8>)> {
        let Some(Value::StrArray(channels)) = item.attributes.get("avatar_channels") else {
            return None;
        };
        if channels.is_empty() {
            return None;
        }

        let mut color = Self::item_icon_color(assets, item, [188, 173, 159, 255]);
        color[3] = 255;
        let (icon, (width, height)) =
            Self::avatar_channel_icon_texture(assets, item, channels, color)?;
        let size = width.max(height).max(1);
        if width == size && height == size {
            return Some((size as u32, icon));
        }

        let mut square = vec![0; size * size * 4];
        let offset_x = (size - width) / 2;
        let offset_y = (size - height) / 2;
        for y in 0..height {
            let src = y * width * 4;
            let dst = ((y + offset_y) * size + offset_x) * 4;
            square[dst..dst + width * 4].copy_from_slice(&icon[src..src + width * 4]);
        }
        Some((size as u32, square))
    }

    fn draw_generated_avatar_channel_icon(
        buffer: &mut TheRGBABuffer,
        rect: Rect,
        assets: &Assets,
        item: &Item,
        draw2d: &Draw2D,
    ) -> bool {
        let Some(Value::StrArray(channels)) = item.attributes.get("avatar_channels") else {
            return false;
        };
        if channels.is_empty() {
            return false;
        }

        let mut color = Self::item_icon_color(assets, item, [188, 173, 159, 255]);
        color[3] = 255;

        let Some((icon, source_size)) =
            Self::avatar_channel_icon_texture(assets, item, channels, color)
        else {
            return false;
        };
        let dest = Self::fit_rect(rect, source_size);
        let stride = buffer.stride();
        draw2d.blend_scale_chunk(buffer.pixels_mut(), &dest, stride, &icon, &source_size);
        true
    }

    fn draw_generated_equipment_icon(
        buffer: &mut TheRGBABuffer,
        rect: Rect,
        assets: &Assets,
        item: &Item,
        draw2d: &Draw2D,
    ) -> bool {
        let Some((size, icon)) = Self::item_equipment_icon_square(assets, item) else {
            return false;
        };
        let dest = Self::fit_rect(rect, (size as usize, size as usize));
        let stride = buffer.stride();
        draw2d.blend_scale_chunk(
            buffer.pixels_mut(),
            &dest,
            stride,
            &icon,
            &(size as usize, size as usize),
        );
        true
    }

    fn item_equipment_icon_square(assets: &Assets, item: &Item) -> Option<(u32, Vec<u8>)> {
        let category = item
            .attributes
            .get_str("category")
            .or_else(|| item.attributes.get_str("ruleset_kind"))
            .or_else(|| item.attributes.get_str("slot"))?
            .trim()
            .to_ascii_lowercase();
        let template = item
            .attributes
            .get_str("icon_template")
            .or_else(|| item.attributes.get_str("visual_template"))
            .or_else(|| item.attributes.get_str("rig_template"))
            .unwrap_or(&category)
            .trim()
            .to_ascii_lowercase();
        let mut blade = Self::item_role_color(
            assets,
            item,
            "blade",
            Self::item_icon_color(assets, item, [187, 195, 208, 255]),
        );
        blade[3] = 255;
        let mut grip = Self::item_role_color(assets, item, "grip", [165, 120, 80, 255]);
        grip[3] = 255;
        let mut accent = Self::item_role_color(assets, item, "accent", [48, 56, 67, 255]);
        accent[3] = 255;
        let mut highlight = Self::item_role_color(assets, item, "highlight", [241, 246, 240, 255]);
        highlight[3] = 255;

        if let Some(icon) = Self::item_template_mask_square(item, blade, grip, accent, highlight) {
            return Some(icon);
        }

        if template == "sword_diagonal" {
            let mut icon = vec![0_u8; 16 * 16 * 4];
            Self::draw_icon_line(&mut icon, 16, 4, 11, 12, 3, blade);
            Self::draw_icon_line(&mut icon, 16, 5, 11, 13, 3, blade);
            Self::draw_icon_pixel(&mut icon, 16, 13, 2, highlight);
            Self::draw_icon_line(&mut icon, 16, 8, 13, 12, 9, accent);
            Self::draw_icon_pixel(&mut icon, 16, 7, 12, accent);
            Self::draw_icon_rect(&mut icon, 16, 2, 13, 3, 2, grip);
            Self::draw_icon_pixel(&mut icon, 16, 1, 15, grip);
            return Some((16, icon));
        }

        let mut icon = vec![0_u8; 24 * 24 * 4];

        match template.as_str() {
            "sword" => {
                Self::draw_icon_line(&mut icon, 24, 7, 17, 16, 8, blade);
                Self::draw_icon_line(&mut icon, 24, 8, 17, 17, 8, blade);
                Self::draw_icon_line(&mut icon, 24, 13, 20, 18, 15, accent);
                Self::draw_icon_rect(&mut icon, 24, 5, 18, 4, 3, grip);
                Self::draw_icon_pixel(&mut icon, 24, 18, 7, highlight);
            }
            "axe" => {
                Self::draw_icon_line(&mut icon, 24, 8, 20, 16, 8, grip);
                Self::draw_icon_rect(&mut icon, 24, 13, 5, 6, 6, blade);
                Self::draw_icon_pixel(&mut icon, 24, 12, 7, blade);
                Self::draw_icon_pixel(&mut icon, 24, 19, 8, blade);
                Self::draw_icon_pixel(&mut icon, 24, 17, 5, highlight);
            }
            "mace" => {
                Self::draw_icon_line(&mut icon, 24, 8, 20, 16, 8, grip);
                Self::draw_icon_rect(&mut icon, 24, 14, 5, 6, 6, blade);
                Self::draw_icon_pixel(&mut icon, 24, 13, 7, blade);
                Self::draw_icon_pixel(&mut icon, 24, 20, 7, blade);
                Self::draw_icon_pixel(&mut icon, 24, 17, 4, highlight);
            }
            "shield" => {
                Self::draw_icon_rect(&mut icon, 24, 7, 5, 10, 12, blade);
                Self::draw_icon_rect(&mut icon, 24, 8, 4, 8, 14, blade);
                Self::draw_icon_rect(&mut icon, 24, 10, 6, 4, 10, grip);
                Self::draw_icon_rect(&mut icon, 24, 11, 5, 2, 12, highlight);
                Self::draw_icon_pixel(&mut icon, 24, 11, 18, blade);
                Self::draw_icon_pixel(&mut icon, 24, 12, 18, blade);
            }
            "bow" => {
                for y in 4..20 {
                    let x = if y < 9 {
                        8
                    } else if y < 15 {
                        7
                    } else {
                        8
                    };
                    Self::draw_icon_pixel(&mut icon, 24, x, y, grip);
                }
                Self::draw_icon_line(&mut icon, 24, 15, 5, 15, 19, highlight);
            }
            _ => return None,
        }

        Some((24, icon))
    }

    fn item_template_mask_square(
        item: &Item,
        blade: [u8; 4],
        grip: [u8; 4],
        accent: [u8; 4],
        highlight: [u8; 4],
    ) -> Option<(u32, Vec<u8>)> {
        let width = item.attributes.get_int("visual_template_width")? as usize;
        let height = item.attributes.get_int("visual_template_height")? as usize;
        let Some(Value::StrArray(rows)) = item.attributes.get("visual_template_pixels") else {
            return None;
        };
        if width == 0 || height == 0 || rows.len() != height {
            return None;
        }

        let size = width.max(height);
        let offset_x = (size - width) / 2;
        let offset_y = (size - height) / 2;
        let mut icon = vec![0_u8; size * size * 4];
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
                let i = ((y + offset_y) * size + x + offset_x) * 4;
                icon[i..i + 4].copy_from_slice(&color);
            }
        }

        Some((size as u32, icon))
    }

    fn avatar_channel_icon_texture(
        assets: &Assets,
        item: &Item,
        channels: &[String],
        color: [u8; 4],
    ) -> Option<(Vec<u8>, (usize, usize))> {
        let avatar = Self::item_icon_avatar(assets, item)?;
        let frame = avatar
            .animations
            .iter()
            .find(|animation| animation.name.eq_ignore_ascii_case("idle"))
            .or_else(|| avatar.animations.first())?
            .perspectives
            .iter()
            .find(|perspective| perspective.direction == crate::AvatarDirection::Front)
            .or_else(|| {
                avatar
                    .animations
                    .iter()
                    .find(|animation| animation.name.eq_ignore_ascii_case("idle"))
                    .or_else(|| avatar.animations.first())
                    .and_then(|animation| animation.perspectives.first())
            })?
            .frames
            .first()?;

        let width = frame.texture.width;
        let height = frame.texture.height;
        let selected = Self::selected_avatar_marker_channels(channels);
        if !selected.iter().any(|selected| *selected) {
            return None;
        }

        let mut min_x = width;
        let mut min_y = height;
        let mut max_x = 0usize;
        let mut max_y = 0usize;
        let mut marker_min_y = [usize::MAX; 9];
        let mut marker_max_y = [0usize; 9];
        let mut mask = vec![None; width * height];

        for (i, pixel) in frame.texture.data.chunks_exact(4).enumerate() {
            if pixel[3] == 0 {
                continue;
            }
            let Some(marker) = Self::avatar_marker_channel(pixel) else {
                continue;
            };
            if !selected[marker] {
                continue;
            }
            let x = i % width;
            let y = i / width;
            mask[i] = Some(marker);
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
            marker_min_y[marker] = marker_min_y[marker].min(y);
            marker_max_y[marker] = marker_max_y[marker].max(y);
        }

        if min_x > max_x || min_y > max_y {
            return None;
        }

        let pad = 1usize;
        let out_width = (max_x - min_x + 1) + pad * 2;
        let out_height = (max_y - min_y + 1) + pad * 2;
        let mut out = vec![0; out_width * out_height * 4];
        let outline = Self::shade_color(color, -64);
        let ramp = Self::build_item_icon_shade_ramp(color);

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let i = y * width + x;
                if mask[i].is_none() {
                    continue;
                }
                let ox = x - min_x + pad;
                let oy = y - min_y + pad;
                for (nx, ny) in [
                    (ox.wrapping_sub(1), oy),
                    (ox + 1, oy),
                    (ox, oy.wrapping_sub(1)),
                    (ox, oy + 1),
                ] {
                    if nx >= out_width || ny >= out_height {
                        continue;
                    }
                    let ni = (ny * out_width + nx) * 4;
                    if out[ni + 3] == 0 {
                        out[ni..ni + 4].copy_from_slice(&outline);
                    }
                }
            }
        }

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let i = y * width + x;
                let Some(marker) = mask[i] else {
                    continue;
                };
                let y0 = marker_min_y[marker];
                let y1 = marker_max_y[marker];
                let local_y = if y0 == usize::MAX || y1 <= y0 {
                    0.5
                } else {
                    (y.saturating_sub(y0)) as f32 / (y1 - y0) as f32
                };
                let shade = Self::item_icon_shade_index(x, y, local_y, marker as u32);
                let ox = x - min_x + pad;
                let oy = y - min_y + pad;
                let oi = (oy * out_width + ox) * 4;
                out[oi..oi + 4].copy_from_slice(&ramp[shade]);
            }
        }

        Some((out, (out_width, out_height)))
    }

    fn item_icon_avatar<'a>(assets: &'a Assets, item: &Item) -> Option<&'a crate::Avatar> {
        item.attributes
            .get_str("icon_avatar")
            .or_else(|| item.attributes.get_str("avatar"))
            .or_else(|| assets.default_avatar.as_deref())
            .and_then(|name| assets.avatars.get(name.trim()))
            .or_else(|| assets.avatars.get("humanoid"))
            .or_else(|| assets.avatars.values().next())
    }

    fn selected_avatar_marker_channels(channels: &[String]) -> [bool; 9] {
        let mut selected = [false; 9];
        for channel in channels {
            match channel.trim().to_ascii_lowercase().as_str() {
                "skin" | "head" | "face" => {
                    selected[0] = true;
                    selected[1] = true;
                }
                "torso" => selected[2] = true,
                "arms" => selected[3] = true,
                "legs" => selected[4] = true,
                "hair" => selected[5] = true,
                "eyes" => selected[6] = true,
                "hands" => selected[7] = true,
                "feet" => selected[8] = true,
                _ => {}
            }
        }
        selected
    }

    fn avatar_marker_channel(pixel: &[u8]) -> Option<usize> {
        match [pixel[0], pixel[1], pixel[2]] {
            [255, 0, 255] => Some(0),
            [200, 0, 200] => Some(1),
            [0, 0, 255] => Some(2),
            [0, 120, 255] => Some(3),
            [0, 255, 0] => Some(4),
            [255, 255, 0] => Some(5),
            [0, 255, 255] => Some(6),
            [255, 128, 0] => Some(7),
            [255, 80, 0] => Some(8),
            _ => None,
        }
    }

    fn fit_rect(rect: Rect, source_size: (usize, usize)) -> (usize, usize, usize, usize) {
        let rect_width = rect.width.max(1.0);
        let rect_height = rect.height.max(1.0);
        let scale = (rect_width / source_size.0.max(1) as f32)
            .min(rect_height / source_size.1.max(1) as f32)
            .max(0.01);
        let width = (source_size.0 as f32 * scale)
            .round()
            .min(rect_width)
            .max(1.0) as usize;
        let height = (source_size.1 as f32 * scale)
            .round()
            .min(rect_height)
            .max(1.0) as usize;
        let x = (rect.x + (rect_width - width as f32) * 0.5)
            .round()
            .max(0.0) as usize;
        let y = (rect.y + (rect_height - height as f32) * 0.5)
            .round()
            .max(0.0) as usize;
        (x, y, width, height)
    }

    fn build_item_icon_shade_ramp(base: [u8; 4]) -> [[u8; 4]; 4] {
        [
            Self::modulate_icon_color(base, 1.18),
            Self::modulate_icon_color(base, 1.0),
            Self::modulate_icon_color(base, 0.82),
            Self::modulate_icon_color(base, 0.64),
        ]
    }

    fn modulate_icon_color(base: [u8; 4], factor: f32) -> [u8; 4] {
        [
            (base[0] as f32 * factor).clamp(0.0, 255.0) as u8,
            (base[1] as f32 * factor).clamp(0.0, 255.0) as u8,
            (base[2] as f32 * factor).clamp(0.0, 255.0) as u8,
            base[3],
        ]
    }

    fn item_icon_shade_index(x: usize, y: usize, local_y: f32, marker_seed: u32) -> usize {
        const BAYER4: [f32; 16] = [
            0.0, 8.0, 2.0, 10.0, 12.0, 4.0, 14.0, 6.0, 3.0, 11.0, 1.0, 9.0, 15.0, 7.0, 13.0, 5.0,
        ];
        let d = BAYER4[(y & 3) * 4 + (x & 3)] / 15.0;
        let bias = (marker_seed % 3) as f32 * 0.03;
        (local_y.clamp(0.0, 1.0) * 2.7 + d * 0.6 + bias).clamp(0.0, 3.0) as usize
    }

    fn draw_icon_pixel(rgba: &mut [u8], width: usize, x: i32, y: i32, color: [u8; 4]) {
        if x < 0 || y < 0 {
            return;
        }
        let x = x as usize;
        let y = y as usize;
        if width == 0 || x >= width || y >= rgba.len() / (width * 4) {
            return;
        }
        let i = (y * width + x) * 4;
        rgba[i..i + 4].copy_from_slice(&color);
    }

    fn draw_icon_rect(
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
                Self::draw_icon_pixel(rgba, width, xx, yy, color);
            }
        }
    }

    fn draw_icon_line(
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
            Self::draw_icon_pixel(rgba, width, x, y, color);
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

    fn item_icon_color(assets: &Assets, item: &Item, fallback: [u8; 4]) -> [u8; 4] {
        if let Some(Value::Color(color)) = item.attributes.get("icon_color") {
            return color.to_u8_array();
        }
        if let Some(hex) = item.attributes.get_str("icon_color") {
            return TheColor::from_hex(hex).to_u8_array();
        }
        if let Some(Value::Color(color)) = item.attributes.get("color") {
            return color.to_u8_array();
        }
        if let Some(hex) = item.attributes.get_str("color") {
            return TheColor::from_hex(hex).to_u8_array();
        }
        if let Some(index) = item
            .attributes
            .get_int("icon_color")
            .or_else(|| item.attributes.get_int("color"))
            .or_else(|| item.attributes.get_int("color_index"))
        {
            return Self::palette_color(assets, index, fallback);
        }
        fallback
    }

    fn item_role_color(assets: &Assets, item: &Item, role: &str, fallback: [u8; 4]) -> [u8; 4] {
        let color_key = format!("{role}_color");
        let index_key = format!("{role}_color_index");
        if let Some(Value::Color(color)) = item.attributes.get(&color_key) {
            return color.to_u8_array();
        }
        if let Some(hex) = item.attributes.get_str(&color_key) {
            return TheColor::from_hex(hex).to_u8_array();
        }
        if let Some(index) = item.attributes.get_int(&color_key) {
            return Self::palette_color(assets, index, fallback);
        }
        if let Some(index) = item.attributes.get_int(&index_key) {
            return Self::palette_color(assets, index, fallback);
        }
        fallback
    }

    fn palette_color(assets: &Assets, index: i32, fallback: [u8; 4]) -> [u8; 4] {
        if index < 0 {
            return fallback;
        }
        let index = index as usize;
        if index < assets.palette.colors.len()
            && let Some(color) = &assets.palette[index]
        {
            return color.to_u8_array();
        }
        fallback
    }

    fn shade_color(color: [u8; 4], delta: i16) -> [u8; 4] {
        [
            (color[0] as i16 + delta).clamp(0, 255) as u8,
            (color[1] as i16 + delta).clamp(0, 255) as u8,
            (color[2] as i16 + delta).clamp(0, 255) as u8,
            color[3],
        ]
    }
}
