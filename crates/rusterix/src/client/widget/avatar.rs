use crate::{
    Assets, AvatarDirection, AvatarShadingOptions, Entity, Rect,
    avatar_builder::AvatarRuntimeBuilder, client::draw2d,
};
use draw2d::Draw2D;
use std::time::Instant;
use theframework::prelude::*;

pub struct AvatarWidget {
    pub name: String,
    pub rect: Rect,
    pub toml_str: String,
    pub buffer: TheRGBABuffer,
    pub party: Option<String>,

    pub avatar_name: Option<String>,
    pub animation: Option<String>,
    pub frame_index: usize,
    pub perspective: AvatarDirection,
    pub show_weapons: bool,

    pub border_color: [u8; 4],
    pub border_color_index: Option<i32>,
    pub damage_border_color: [u8; 4],
    pub damage_border_color_index: Option<i32>,
    pub border_size: usize,
    pub damage_flash_seconds: f32,
    pub damage_stat: String,

    pub(crate) last_stat_value: Option<f32>,
    pub(crate) damage_flash_started: Option<Instant>,
}

impl Default for AvatarWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl AvatarWidget {
    pub fn new() -> Self {
        Self {
            name: String::new(),
            rect: Rect::default(),
            toml_str: String::new(),
            buffer: TheRGBABuffer::default(),
            party: None,
            avatar_name: None,
            animation: None,
            frame_index: 0,
            perspective: AvatarDirection::Front,
            show_weapons: true,
            border_color: [172, 182, 194, 190],
            border_color_index: None,
            damage_border_color: [224, 67, 64, 235],
            damage_border_color_index: None,
            border_size: 2,
            damage_flash_seconds: 0.8,
            damage_stat: "HP".to_string(),
            last_stat_value: None,
            damage_flash_started: None,
        }
    }

    fn color_from_ui(
        ui: &toml::value::Table,
        flat_key: &str,
        style_state: Option<&str>,
        style_key: &str,
    ) -> Option<[u8; 4]> {
        if let Some(color) = ui
            .get(flat_key)
            .and_then(toml::Value::as_str)
            .map(TheColor::from_hex)
        {
            return Some(color.to_u8_array());
        }

        let style = ui.get("style").and_then(toml::Value::as_table)?;
        match style_state {
            Some(state) => style
                .get(state)
                .and_then(toml::Value::as_table)
                .and_then(|state_style| state_style.get(style_key))
                .and_then(toml::Value::as_str)
                .map(TheColor::from_hex)
                .map(|color| color.to_u8_array()),
            None => style
                .get(style_key)
                .and_then(toml::Value::as_str)
                .map(TheColor::from_hex)
                .map(|color| color.to_u8_array()),
        }
    }

    fn int_from_ui(
        ui: &toml::value::Table,
        flat_key: &str,
        style_state: Option<&str>,
        style_key: &str,
    ) -> Option<i32> {
        if let Some(value) = ui
            .get(flat_key)
            .and_then(toml::Value::as_integer)
            .and_then(|value| i32::try_from(value).ok())
        {
            return Some(value);
        }

        let style = ui.get("style").and_then(toml::Value::as_table)?;
        match style_state {
            Some(state) => style
                .get(state)
                .and_then(toml::Value::as_table)
                .and_then(|state_style| state_style.get(style_key))
                .and_then(toml::Value::as_integer)
                .and_then(|value| i32::try_from(value).ok()),
            None => style
                .get(style_key)
                .and_then(toml::Value::as_integer)
                .and_then(|value| i32::try_from(value).ok()),
        }
    }

    fn float_from_ui(ui: &toml::value::Table, key: &str) -> Option<f32> {
        ui.get(key).and_then(|value| {
            value
                .as_float()
                .map(|value| value as f32)
                .or_else(|| value.as_integer().map(|value| value as f32))
        })
    }

    pub fn init(&mut self) {
        if let Ok(value) = self.toml_str.parse::<toml::Table>()
            && let Some(ui) = value.get("ui").and_then(toml::Value::as_table)
        {
            if let Some(v) = ui.get("avatar").and_then(toml::Value::as_str) {
                let name = v.trim();
                if !name.is_empty() {
                    self.avatar_name = Some(name.to_string());
                }
            }

            if let Some(v) = ui.get("party").and_then(toml::Value::as_str) {
                let binding = v.trim();
                if !binding.is_empty() {
                    self.party = Some(binding.to_string());
                }
            }

            if let Some(v) = ui.get("animation").and_then(toml::Value::as_str) {
                let anim = v.trim();
                if !anim.is_empty() {
                    self.animation = Some(anim.to_string());
                }
            }

            if let Some(v) = ui.get("frame_index").and_then(toml::Value::as_integer)
                && v >= 0
            {
                self.frame_index = v as usize;
            }

            if let Some(v) = ui.get("perspective").and_then(toml::Value::as_str) {
                self.perspective = AvatarDirection::from_key(v).unwrap_or(AvatarDirection::Front);
            }

            if let Some(v) = ui.get("show_weapons").and_then(toml::Value::as_bool) {
                self.show_weapons = v;
            }

            if let Some(color) = Self::color_from_ui(ui, "border_color", None, "border") {
                self.border_color = color;
            }
            self.border_color_index =
                Self::int_from_ui(ui, "border_color_index", None, "border_index")
                    .or_else(|| Self::int_from_ui(ui, "border_index", None, "border_index"));

            if let Some(color) =
                Self::color_from_ui(ui, "damage_border_color", Some("damage"), "border")
            {
                self.damage_border_color = color;
            }
            self.damage_border_color_index = Self::int_from_ui(
                ui,
                "damage_border_color_index",
                Some("damage"),
                "border_index",
            )
            .or_else(|| {
                Self::int_from_ui(ui, "damage_border_index", Some("damage"), "border_index")
            });

            if let Some(v) = ui.get("border_size").and_then(toml::Value::as_integer)
                && v >= 0
            {
                self.border_size = v as usize;
            }

            if let Some(v) = Self::float_from_ui(ui, "damage_flash_seconds") {
                self.damage_flash_seconds = v.max(0.0);
            }

            if let Some(v) = ui.get("damage_stat").and_then(toml::Value::as_str) {
                let stat = v.trim();
                if !stat.is_empty() {
                    self.damage_stat = stat.to_string();
                }
            }
        }
    }

    fn palette_color(assets: &Assets, index: Option<i32>, fallback: [u8; 4]) -> [u8; 4] {
        let Some(index) = index else {
            return fallback;
        };
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

    fn update_damage_state(&mut self, entity: &Entity) {
        let current = entity.attributes.get_float(&self.damage_stat);
        if let Some(current) = current {
            if let Some(previous) = self.last_stat_value
                && current < previous
                && self.damage_flash_seconds > 0.0
            {
                self.damage_flash_started = Some(Instant::now());
            }
            self.last_stat_value = Some(current);
        }

        if self
            .damage_flash_started
            .is_some_and(|started| started.elapsed().as_secs_f32() >= self.damage_flash_seconds)
        {
            self.damage_flash_started = None;
        }
    }

    pub(crate) fn draw_alpha_outline(buffer: &mut TheRGBABuffer, color: [u8; 4], thickness: usize) {
        if thickness == 0 || color[3] == 0 {
            return;
        }

        let width = buffer.dim().width as usize;
        let height = buffer.dim().height as usize;
        if width == 0 || height == 0 {
            return;
        }

        let pixels = buffer.pixels_mut();
        let alpha: Vec<u8> = pixels.chunks_exact(4).map(|px| px[3]).collect();
        let mut outline = vec![false; width * height];
        let radius = thickness as isize;

        for y in 0..height {
            for x in 0..width {
                let idx = x + y * width;
                if alpha[idx] == 0 {
                    continue;
                }

                for dy in -radius..=radius {
                    for dx in -radius..=radius {
                        if dx == 0 && dy == 0 {
                            continue;
                        }
                        if dx * dx + dy * dy > radius * radius {
                            continue;
                        }
                        let nx = x as isize + dx;
                        let ny = y as isize + dy;
                        if nx < 0 || ny < 0 || nx >= width as isize || ny >= height as isize {
                            continue;
                        }
                        let nidx = nx as usize + ny as usize * width;
                        if alpha[nidx] == 0 {
                            outline[nidx] = true;
                        }
                    }
                }
            }
        }

        for (idx, is_outline) in outline.into_iter().enumerate() {
            if !is_outline {
                continue;
            }
            let offset = idx * 4;
            pixels[offset..offset + 4].copy_from_slice(&color);
        }
    }

    fn draw_border(&mut self, assets: &Assets, _draw2d: &Draw2D) {
        if self.border_size == 0 {
            return;
        }

        let damage_active = self.damage_flash_started.is_some();
        let color = if damage_active {
            Self::palette_color(
                assets,
                self.damage_border_color_index,
                self.damage_border_color,
            )
        } else {
            Self::palette_color(assets, self.border_color_index, self.border_color)
        };
        if color[3] == 0 {
            return;
        }

        Self::draw_alpha_outline(&mut self.buffer, color, self.border_size);
    }

    fn finish_draw(&mut self, target: &mut TheRGBABuffer, assets: &Assets, draw2d: &Draw2D) {
        self.draw_border(assets, draw2d);
        target.blend_into(self.rect.x as i32, self.rect.y as i32, &self.buffer);
    }

    pub fn update_draw(
        &mut self,
        target: &mut TheRGBABuffer,
        assets: &Assets,
        entity: Option<&Entity>,
        draw2d: &Draw2D,
    ) {
        self.buffer.fill([0, 0, 0, 0]);

        let Some(entity) = entity else {
            target.blend_into(self.rect.x as i32, self.rect.y as i32, &self.buffer);
            return;
        };
        self.update_damage_state(entity);

        let Some(avatar) = self
            .avatar_name
            .as_ref()
            .and_then(|name| assets.avatars.get(name))
            .or_else(|| AvatarRuntimeBuilder::find_avatar_for_entity(entity, assets))
        else {
            self.finish_draw(target, assets, draw2d);
            return;
        };

        let Some(out) = AvatarRuntimeBuilder::build_preview_for_entity_with_weapons(
            entity,
            avatar,
            assets,
            self.animation.as_deref(),
            self.perspective,
            self.frame_index,
            AvatarShadingOptions::default(),
            self.show_weapons,
        ) else {
            self.finish_draw(target, assets, draw2d);
            return;
        };

        let stride = self.buffer.stride();
        let dst_w = self.buffer.dim().width as usize;
        let dst_h = self.buffer.dim().height as usize;
        draw2d.blend_scale_chunk(
            self.buffer.pixels_mut(),
            &(0, 0, dst_w, dst_h),
            stride,
            &out.rgba,
            &(out.size as usize, out.size as usize),
        );
        self.finish_draw(target, assets, draw2d);
    }
}
