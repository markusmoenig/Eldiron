use crate::{
    Assets, AvatarDirection, AvatarShadingOptions, Entity, Rect, Texture,
    avatar_builder::AvatarRuntimeBuilder, client::draw2d,
};
use draw2d::Draw2D;
use theframework::prelude::*;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ProfileCrop {
    Bust,
    Face,
    Full,
}

impl Default for ProfileCrop {
    fn default() -> Self {
        Self::Bust
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ProfileStatsLayout {
    Side,
    Bottom,
    Vertical,
}

impl Default for ProfileStatsLayout {
    fn default() -> Self {
        Self::Side
    }
}

#[derive(Clone)]
struct ProfileStatBar {
    stat: String,
    max_stat: String,
    background_color: [u8; 4],
    fill_color: [u8; 4],
    border_color: [u8; 4],
    border_size: usize,
    height: f32,
    width: f32,
}

impl ProfileStatBar {
    fn new(
        stat: &str,
        fill_color: [u8; 4],
        background_color: [u8; 4],
        border_color: [u8; 4],
    ) -> Self {
        Self {
            stat: stat.to_string(),
            max_stat: format!("MAX_{stat}"),
            background_color,
            fill_color,
            border_color,
            border_size: 0,
            height: 12.0,
            width: 10.0,
        }
    }
}

pub struct ProfileWidget {
    pub name: String,
    pub rect: Rect,
    pub toml_str: String,
    pub buffer: TheRGBABuffer,
    pub party: Option<String>,

    avatar_name: Option<String>,
    animation: Option<String>,
    frame_index: usize,
    perspective: AvatarDirection,
    show_weapons: bool,
    crop: ProfileCrop,

    border_color: [u8; 4],
    border_size: usize,
    name_color: [u8; 4],
    class_color: [u8; 4],
    name_font_size: f32,
    class_font_size: f32,
    text_gap: f32,
    image_size: Option<f32>,
    show_text: bool,
    selected_frame: bool,
    selected_frame_color: [u8; 4],
    selected_frame_size: usize,
    selected_frame_padding: usize,
    stat_bars: Vec<ProfileStatBar>,
    stats_layout: ProfileStatsLayout,
    stat_gap: f32,
    stats_top_gap: f32,
}

impl Default for ProfileWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl ProfileWidget {
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
            show_weapons: false,
            crop: ProfileCrop::Bust,
            border_color: [172, 182, 194, 190],
            border_size: 1,
            name_color: [240, 240, 240, 255],
            class_color: [126, 228, 232, 255],
            name_font_size: 16.0,
            class_font_size: 13.0,
            text_gap: 4.0,
            image_size: None,
            show_text: true,
            selected_frame: false,
            selected_frame_color: [69, 203, 208, 255],
            selected_frame_size: 2,
            selected_frame_padding: 4,
            stat_bars: Vec::new(),
            stats_layout: ProfileStatsLayout::default(),
            stat_gap: 8.0,
            stats_top_gap: 8.0,
        }
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
                let animation = v.trim();
                if !animation.is_empty() {
                    self.animation = Some(animation.to_string());
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
            if let Some(v) = ui.get("crop").and_then(toml::Value::as_str) {
                self.crop = match v.trim().to_ascii_lowercase().as_str() {
                    "face" | "head" => ProfileCrop::Face,
                    "full" | "avatar" => ProfileCrop::Full,
                    _ => ProfileCrop::Bust,
                };
            }
            if let Some(color) = ui
                .get("border_color")
                .and_then(toml::Value::as_str)
                .map(TheColor::from_hex)
            {
                self.border_color = color.to_u8_array();
            }
            if let Some(v) = ui.get("border_size").and_then(toml::Value::as_integer)
                && v >= 0
            {
                self.border_size = v as usize;
            }
            if let Some(color) = ui
                .get("name_color")
                .or_else(|| ui.get("color"))
                .and_then(toml::Value::as_str)
                .map(TheColor::from_hex)
            {
                self.name_color = color.to_u8_array();
            }
            if let Some(color) = ui
                .get("class_color")
                .or_else(|| ui.get("subtitle_color"))
                .and_then(toml::Value::as_str)
                .map(TheColor::from_hex)
            {
                self.class_color = color.to_u8_array();
            }
            if let Some(v) = Self::float_from_ui(ui, "name_font_size")
                .or_else(|| Self::float_from_ui(ui, "font_size"))
            {
                self.name_font_size = v.max(1.0);
            }
            if let Some(v) = Self::float_from_ui(ui, "class_font_size") {
                self.class_font_size = v.max(1.0);
            }
            if let Some(v) = Self::float_from_ui(ui, "text_gap") {
                self.text_gap = v.max(0.0);
            }
            if let Some(v) = Self::float_from_ui(ui, "image_size")
                .or_else(|| Self::float_from_ui(ui, "profile_size"))
            {
                self.image_size = Some(v.max(1.0));
            }
            if let Some(v) = ui.get("show_text").and_then(toml::Value::as_bool) {
                self.show_text = v;
            }
            if let Some(v) = ui
                .get("selected_frame")
                .or_else(|| ui.get("selected"))
                .and_then(toml::Value::as_bool)
            {
                self.selected_frame = v;
            }
            if let Some(color) = ui
                .get("selected_frame_color")
                .or_else(|| ui.get("frame_color"))
                .and_then(toml::Value::as_str)
                .map(TheColor::from_hex)
            {
                self.selected_frame_color = color.to_u8_array();
            }
            if let Some(v) = ui
                .get("selected_frame_size")
                .or_else(|| ui.get("frame_size"))
                .and_then(toml::Value::as_integer)
                && v >= 0
            {
                self.selected_frame_size = v as usize;
            }
            if let Some(v) = ui
                .get("selected_frame_padding")
                .or_else(|| ui.get("frame_padding"))
                .and_then(toml::Value::as_integer)
                && v >= 0
            {
                self.selected_frame_padding = v as usize;
            }
            if let Some(v) = ui
                .get("stats_layout")
                .or_else(|| ui.get("stat_layout"))
                .and_then(toml::Value::as_str)
            {
                self.stats_layout = match v.trim().to_ascii_lowercase().as_str() {
                    "bottom" | "full" | "full_width" => ProfileStatsLayout::Bottom,
                    "vertical" | "meters" | "dm" => ProfileStatsLayout::Vertical,
                    _ => ProfileStatsLayout::Side,
                };
            }
            if let Some(v) = Self::float_from_ui(ui, "stat_gap") {
                self.stat_gap = v.max(0.0);
            }
            if let Some(v) = Self::float_from_ui(ui, "stats_top_gap") {
                self.stats_top_gap = v.max(0.0);
            }
            self.stat_bars = Self::parse_stat_bars(ui);
        }
    }

    fn parse_stat_bars(ui: &toml::value::Table) -> Vec<ProfileStatBar> {
        let Some(stats) = ui.get("stats").and_then(toml::Value::as_array) else {
            return Vec::new();
        };
        stats
            .iter()
            .filter_map(|value| value.as_table())
            .filter_map(|table| {
                let stat = table
                    .get("stat")
                    .or_else(|| table.get("attribute"))
                    .or_else(|| table.get("value"))
                    .and_then(toml::Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())?;
                let mut bar = ProfileStatBar::new(
                    stat,
                    Self::color_from_table(table, &["fill_color", "color"], [196, 48, 48, 255]),
                    Self::color_from_table(
                        table,
                        &["background_color", "background"],
                        [0, 0, 0, 0],
                    ),
                    Self::color_from_table(table, &["border_color"], [255, 255, 255, 255]),
                );
                bar.max_stat = table
                    .get("max_stat")
                    .or_else(|| table.get("max_attribute"))
                    .or_else(|| table.get("max"))
                    .and_then(toml::Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_string)
                    .unwrap_or_else(|| format!("MAX_{}", bar.stat));
                if let Some(v) = Self::float_from_table(table, "height") {
                    bar.height = v.max(1.0);
                }
                if let Some(v) = Self::float_from_table(table, "width") {
                    bar.width = v.max(1.0);
                }
                if let Some(v) = table.get("border_size").and_then(toml::Value::as_integer)
                    && v >= 0
                {
                    bar.border_size = v as usize;
                }
                Some(bar)
            })
            .collect()
    }

    fn float_from_ui(ui: &toml::value::Table, key: &str) -> Option<f32> {
        ui.get(key).and_then(|value| {
            value
                .as_float()
                .map(|value| value as f32)
                .or_else(|| value.as_integer().map(|value| value as f32))
        })
    }

    fn float_from_table(table: &toml::value::Table, key: &str) -> Option<f32> {
        table.get(key).and_then(|value| {
            value
                .as_float()
                .map(|value| value as f32)
                .or_else(|| value.as_integer().map(|value| value as f32))
        })
    }

    fn color_from_table(table: &toml::value::Table, keys: &[&str], fallback: [u8; 4]) -> [u8; 4] {
        keys.iter()
            .find_map(|key| table.get(*key))
            .and_then(toml::Value::as_str)
            .map(TheColor::from_hex)
            .map(|color| color.to_u8_array())
            .unwrap_or(fallback)
    }

    fn explicit_profile_texture(entity: &Entity, assets: &Assets) -> Option<Texture> {
        for key in ["profile_tile_id", "profile_source", "portrait_tile_id"] {
            if let Some(source) = entity.attributes.get_source(key)
                && let Some(tile) = source.tile_from_tile_list(assets)
                && let Some(texture) = tile.textures.first()
            {
                return Some(texture.clone());
            }
            if let Some(id) = entity.attributes.get_id(key)
                && let Some(tile) = assets.tiles.get(&id)
                && let Some(texture) = tile.textures.first()
            {
                return Some(texture.clone());
            }
            if let Some(id) = entity
                .attributes
                .get_str(key)
                .and_then(|value| Uuid::parse_str(value.trim()).ok())
                && let Some(tile) = assets.tiles.get(&id)
                && let Some(texture) = tile.textures.first()
            {
                return Some(texture.clone());
            }
        }
        None
    }

    fn alpha_bounds(size: usize, rgba: &[u8]) -> Option<(usize, usize, usize, usize)> {
        if size == 0 || rgba.len() < size * size * 4 {
            return None;
        }
        let mut min_x = size;
        let mut min_y = size;
        let mut max_x = 0usize;
        let mut max_y = 0usize;
        for y in 0..size {
            for x in 0..size {
                if rgba[(y * size + x) * 4 + 3] == 0 {
                    continue;
                }
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x + 1);
                max_y = max_y.max(y + 1);
            }
        }
        (min_x < max_x && min_y < max_y).then_some((min_x, min_y, max_x, max_y))
    }

    fn crop_rect(
        size: usize,
        rgba: &[u8],
        crop: ProfileCrop,
    ) -> Option<(usize, usize, usize, usize)> {
        let (min_x, min_y, max_x, max_y) = Self::alpha_bounds(size, rgba)?;
        if crop == ProfileCrop::Full {
            return Some((min_x, min_y, max_x - min_x, max_y - min_y));
        }

        let body_w = (max_x - min_x).max(1) as f32;
        let body_h = (max_y - min_y).max(1) as f32;
        let center_x = (min_x + max_x) as f32 * 0.5;
        let (height_factor, width_factor, top_margin) = match crop {
            ProfileCrop::Face => (0.36, 0.82, 0.02),
            ProfileCrop::Bust | ProfileCrop::Full => (0.46, 1.08, 0.02),
        };
        let crop_h = (body_h * height_factor).round().max(1.0);
        let crop_w = (body_w * width_factor).round().max(crop_h * 0.78).max(1.0);
        let crop_y = min_y as f32 - body_h * top_margin;
        let crop_x = center_x - crop_w * 0.5;

        Some(Self::clamped_rect(size, crop_x, crop_y, crop_w, crop_h))
    }

    fn clamped_rect(
        size: usize,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    ) -> (usize, usize, usize, usize) {
        let mut x0 = x.floor() as isize;
        let mut y0 = y.floor() as isize;
        let mut w = width.ceil().max(1.0) as isize;
        let mut h = height.ceil().max(1.0) as isize;
        if x0 < 0 {
            w += x0;
            x0 = 0;
        }
        if y0 < 0 {
            h += y0;
            y0 = 0;
        }
        if x0 + w > size as isize {
            w = size as isize - x0;
        }
        if y0 + h > size as isize {
            h = size as isize - y0;
        }
        (
            x0 as usize,
            y0 as usize,
            w.max(1) as usize,
            h.max(1) as usize,
        )
    }

    fn cropped_rgba(
        rgba: &[u8],
        size: usize,
        rect: (usize, usize, usize, usize),
    ) -> Option<Vec<u8>> {
        let (x, y, w, h) = rect;
        if size == 0 || w == 0 || h == 0 || rgba.len() < size * size * 4 {
            return None;
        }
        let mut out = vec![0u8; w * h * 4];
        for row in 0..h {
            let src = ((y + row) * size + x) * 4;
            let dst = row * w * 4;
            out[dst..dst + w * 4].copy_from_slice(&rgba[src..src + w * 4]);
        }
        Some(out)
    }

    fn draw_border(&mut self) {
        if self.border_size == 0 || self.border_color[3] == 0 {
            return;
        }
        super::avatar::AvatarWidget::draw_alpha_outline(
            &mut self.buffer,
            self.border_color,
            self.border_size,
        );
    }

    fn draw_selected_frame(&mut self, draw2d: &Draw2D) {
        if !self.selected_frame
            || self.selected_frame_size == 0
            || self.selected_frame_color[3] == 0
        {
            return;
        }

        let w = self.buffer.dim().width.max(0) as usize;
        let h = self.buffer.dim().height.max(0) as usize;
        if w == 0 || h == 0 {
            return;
        }

        let stride = self.buffer.stride();
        draw2d.rect_outline_thickness(
            self.buffer.pixels_mut(),
            &(0, 0, w, h),
            stride,
            &self.selected_frame_color,
            self.selected_frame_size,
        );
    }

    fn finish_draw(&mut self, target: &mut TheRGBABuffer, draw2d: &Draw2D) {
        self.draw_selected_frame(draw2d);
        target.blend_into(self.rect.x as i32, self.rect.y as i32, &self.buffer);
    }

    fn content_inset(&self) -> usize {
        if self.selected_frame && self.selected_frame_size > 0 && self.selected_frame_color[3] > 0 {
            self.selected_frame_size + self.selected_frame_padding
        } else {
            0
        }
    }

    fn offset_layout(
        image_rect: &mut (usize, usize, usize, usize),
        text_rect: &mut Option<Rect>,
        stat_rects: &mut [(usize, usize, usize, usize)],
        inset: usize,
    ) {
        if inset == 0 {
            return;
        }
        image_rect.0 += inset;
        image_rect.1 += inset;
        if let Some(rect) = text_rect {
            rect.x += inset as f32;
            rect.y += inset as f32;
        }
        for rect in stat_rects {
            rect.0 += inset;
            rect.1 += inset;
        }
    }

    fn fallback_font() -> Option<fontdue::Font> {
        fontdue::Font::from_bytes(
            include_bytes!("../../../../theframework/embedded/fonts/Roboto-Bold.ttf").as_slice(),
            fontdue::FontSettings::default(),
        )
        .ok()
    }

    fn display_name(entity: &Entity) -> String {
        entity
            .attributes
            .get_str("display_name")
            .or_else(|| entity.attributes.get_str("name"))
            .or_else(|| entity.attributes.get_str("source_id"))
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(Self::title_from_identifier)
            .unwrap_or_else(|| {
                if entity.is_player() {
                    "Player".to_string()
                } else {
                    "Unknown".to_string()
                }
            })
    }

    fn display_class(entity: &Entity) -> Option<String> {
        entity
            .attributes
            .get_str("class")
            .or_else(|| entity.attributes.get_str("class_name"))
            .or_else(|| entity.attributes.get_str("ruleset_class"))
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(Self::title_from_identifier)
    }

    fn title_from_identifier(value: &str) -> String {
        value
            .split(['_', '-'])
            .filter(|part| !part.is_empty())
            .map(|part| {
                let mut chars = part.chars();
                match chars.next() {
                    Some(first) => first.to_uppercase().chain(chars).collect::<String>(),
                    None => String::new(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn stat_bars_height(&self) -> f32 {
        if self.stat_bars.is_empty() {
            return 0.0;
        }
        self.stat_bars
            .iter()
            .map(|bar| bar.height.max(1.0))
            .sum::<f32>()
            + self.stat_gap * self.stat_bars.len().saturating_sub(1) as f32
            + self.stats_top_gap
    }

    fn layout(
        &self,
        dst_w: usize,
        dst_h: usize,
    ) -> (
        (usize, usize, usize, usize),
        Option<Rect>,
        Vec<(usize, usize, usize, usize)>,
    ) {
        let w = dst_w as f32;
        let h = dst_h as f32;
        if self.stats_layout == ProfileStatsLayout::Bottom {
            return self.bottom_layout(dst_w, dst_h);
        }

        let image = self
            .image_size
            .unwrap_or_else(|| (h * 0.86).min(w * 0.42))
            .min(h)
            .max(1.0);
        let image_rect = (
            0,
            ((h - image) * 0.5).round() as usize,
            image.round() as usize,
            image.round() as usize,
        );
        let mut next_x = image + self.text_gap + 4.0;
        let stat_rects = if self.stats_layout == ProfileStatsLayout::Vertical {
            next_x += self.stats_top_gap;
            let mut bar_x = next_x;
            let rects = self
                .stat_bars
                .iter()
                .map(|bar| {
                    let rect = (
                        bar_x.round() as usize,
                        0,
                        bar.width.round().max(1.0) as usize,
                        dst_h,
                    );
                    bar_x += bar.width.max(1.0) + self.stat_gap;
                    rect
                })
                .filter(|(x, _, width, height)| *x < dst_w && *width > 0 && *height > 0)
                .map(|(x, y, width, height)| {
                    let clamped_width = width.min(dst_w.saturating_sub(x));
                    (x, y, clamped_width, height)
                })
                .collect::<Vec<_>>();
            next_x = bar_x + 4.0;
            rects
        } else {
            let stats_x = next_x;
            let stats_w = (w - stats_x).max(1.0);
            let text_bottom = self.name_font_size + self.text_gap + self.class_font_size;
            let bar_count = self.stat_bars.len();
            let total_bar_height = self
                .stat_bars
                .iter()
                .map(|bar| bar.height.max(1.0))
                .sum::<f32>();
            let desired_gap_space =
                self.stats_top_gap + self.stat_gap * bar_count.saturating_sub(1) as f32;
            let available_gap_space = (h - text_bottom - total_bar_height).max(0.0);
            let gap_scale = if desired_gap_space > available_gap_space && desired_gap_space > 0.0 {
                available_gap_space / desired_gap_space
            } else {
                1.0
            };
            let top_gap = self.stats_top_gap * gap_scale;
            let stat_gap = self.stat_gap * gap_scale;
            let mut bar_y = text_bottom + top_gap;
            self.stat_bars
                .iter()
                .map(|bar| {
                    let rect = (
                        stats_x.round() as usize,
                        bar_y.round() as usize,
                        stats_w.round() as usize,
                        bar.height.round().max(1.0) as usize,
                    );
                    bar_y += bar.height.max(1.0) + stat_gap;
                    rect
                })
                .filter(|(x, y, width, height)| {
                    *x < dst_w && *y < dst_h && *width > 0 && *height > 0
                })
                .map(|(x, y, width, height)| {
                    let clamped_width = width.min(dst_w.saturating_sub(x));
                    let clamped_height = height.min(dst_h.saturating_sub(y));
                    (x, y, clamped_width, clamped_height)
                })
                .collect::<Vec<_>>()
        };

        let text_rect = if self.show_text && next_x < w - 1.0 {
            let text_h = if self.stats_layout == ProfileStatsLayout::Vertical {
                h
            } else {
                (self.name_font_size + self.text_gap + self.class_font_size)
                    .min(h)
                    .max(1.0)
            };
            Some(Rect::new(next_x, 0.0, (w - next_x).max(1.0), text_h))
        } else {
            None
        };

        (image_rect, text_rect, stat_rects)
    }

    fn bottom_layout(
        &self,
        dst_w: usize,
        dst_h: usize,
    ) -> (
        (usize, usize, usize, usize),
        Option<Rect>,
        Vec<(usize, usize, usize, usize)>,
    ) {
        let w = dst_w as f32;
        let h = dst_h as f32;
        let stat_h = self.stat_bars_height().min((h - 1.0).max(0.0));
        let content_h = (h - stat_h).max(1.0);
        let stat_y = content_h + self.stats_top_gap.min(stat_h);
        let mut bar_y = stat_y;
        let stat_rects = self
            .stat_bars
            .iter()
            .map(|bar| {
                let rect = (
                    0,
                    bar_y.round() as usize,
                    dst_w,
                    bar.height.round().max(1.0) as usize,
                );
                bar_y += bar.height.max(1.0) + self.stat_gap;
                rect
            })
            .filter(|(_, y, _, height)| *y < dst_h && *height > 0)
            .map(|(x, y, width, height)| {
                let clamped_height = height.min(dst_h.saturating_sub(y));
                (x, y, width, clamped_height)
            })
            .collect::<Vec<_>>();

        let horizontal = self.show_text && w >= content_h * 1.45;
        if horizontal {
            let image = self
                .image_size
                .unwrap_or_else(|| (content_h * 0.86).min(w * 0.42))
                .min(content_h)
                .max(1.0);
            let image_x = 0.0;
            let image_y = ((content_h - image) * 0.5).max(0.0);
            let text_x = image_x + image + self.text_gap + 4.0;
            let text_w = (w - text_x).max(1.0);
            let text_h = (self.name_font_size + self.class_font_size + self.text_gap)
                .min(content_h)
                .max(1.0);
            let text_y = ((content_h - text_h) * 0.5).max(0.0);
            (
                (
                    image_x.round() as usize,
                    image_y.round() as usize,
                    image.round() as usize,
                    image.round() as usize,
                ),
                Some(Rect::new(text_x, text_y, text_w, text_h)),
                stat_rects,
            )
        } else if self.show_text {
            let text_h = (self.name_font_size + self.class_font_size + self.text_gap + 4.0)
                .min(content_h * 0.45)
                .max(1.0);
            let image = self
                .image_size
                .unwrap_or_else(|| w.min(content_h - text_h))
                .min(w)
                .min((content_h - text_h).max(1.0))
                .max(1.0);
            (
                (
                    ((w - image) * 0.5).round() as usize,
                    0,
                    image.round() as usize,
                    image.round() as usize,
                ),
                Some(Rect::new(
                    0.0,
                    image + 2.0,
                    w,
                    (content_h - image - 2.0).max(1.0),
                )),
                stat_rects,
            )
        } else {
            ((0, 0, dst_w, content_h.round() as usize), None, stat_rects)
        }
    }

    fn draw_identity_text(
        &mut self,
        entity: &Entity,
        assets: &Assets,
        draw2d: &Draw2D,
        text_rect: Rect,
    ) {
        if !self.show_text || text_rect.width <= 1.0 || text_rect.height <= 1.0 {
            return;
        }
        let fallback = Self::fallback_font();
        let Some(font) = assets.fonts.values().next().or(fallback.as_ref()) else {
            return;
        };

        let stride = self.buffer.stride();
        let safe_rect = (
            0,
            0,
            self.buffer.dim().width as isize,
            self.buffer.dim().height as isize,
        );
        let name = Self::display_name(entity);
        let class = Self::display_class(entity).unwrap_or_default();
        let total_h = if class.is_empty() {
            self.name_font_size
        } else {
            self.name_font_size + self.text_gap + self.class_font_size
        };
        let start_y = (text_rect.y + (text_rect.height - total_h) * 0.5).max(text_rect.y);
        draw2d.text_rect_blend_safe(
            self.buffer.pixels_mut(),
            &(
                text_rect.x.floor() as isize,
                start_y.floor() as isize,
                text_rect.width.ceil() as isize,
                self.name_font_size.ceil() as isize,
            ),
            stride,
            font,
            self.name_font_size,
            &name,
            &self.name_color,
            draw2d::TheHorizontalAlign::Left,
            draw2d::TheVerticalAlign::Center,
            &safe_rect,
        );
        if !class.is_empty() {
            draw2d.text_rect_blend_safe(
                self.buffer.pixels_mut(),
                &(
                    text_rect.x.floor() as isize,
                    (start_y + self.name_font_size + self.text_gap).floor() as isize,
                    text_rect.width.ceil() as isize,
                    self.class_font_size.ceil() as isize,
                ),
                stride,
                font,
                self.class_font_size,
                &class,
                &self.class_color,
                draw2d::TheHorizontalAlign::Left,
                draw2d::TheVerticalAlign::Center,
                &safe_rect,
            );
        }
    }

    fn draw_stat_bars(
        &mut self,
        entity: &Entity,
        draw2d: &Draw2D,
        stat_rects: &[(usize, usize, usize, usize)],
    ) {
        let stride = self.buffer.stride();
        let vertical = self.stats_layout == ProfileStatsLayout::Vertical;
        for (bar, rect) in self.stat_bars.iter().zip(stat_rects.iter()) {
            let current = entity.attributes.get_float_default(&bar.stat, 0.0);
            let max = entity
                .attributes
                .get_float_default(&bar.max_stat, current.max(1.0));
            let ratio = if max <= 0.0 {
                0.0
            } else {
                (current / max).clamp(0.0, 1.0)
            };

            if bar.background_color[3] > 0 {
                draw2d.blend_rect(
                    self.buffer.pixels_mut(),
                    rect,
                    stride,
                    &bar.background_color,
                );
            }

            if bar.fill_color[3] > 0 {
                if vertical {
                    let fill_h = ((rect.3 as f32) * ratio).round() as usize;
                    if fill_h > 0 {
                        draw2d.blend_rect(
                            self.buffer.pixels_mut(),
                            &(
                                rect.0,
                                rect.1 + rect.3.saturating_sub(fill_h),
                                rect.2,
                                fill_h,
                            ),
                            stride,
                            &bar.fill_color,
                        );
                    }
                } else {
                    let fill_w = ((rect.2 as f32) * ratio).round() as usize;
                    if fill_w > 0 {
                        draw2d.blend_rect(
                            self.buffer.pixels_mut(),
                            &(rect.0, rect.1, fill_w.min(rect.2), rect.3),
                            stride,
                            &bar.fill_color,
                        );
                    }
                }
            }

            if bar.border_size > 0 && bar.border_color[3] > 0 {
                draw2d.rect_outline_thickness(
                    self.buffer.pixels_mut(),
                    rect,
                    stride,
                    &bar.border_color,
                    bar.border_size,
                );
            }
        }
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
            self.finish_draw(target, draw2d);
            return;
        };

        let stride = self.buffer.stride();
        let dst_w = self.buffer.dim().width.max(0) as usize;
        let dst_h = self.buffer.dim().height.max(0) as usize;
        if dst_w == 0 || dst_h == 0 {
            return;
        }
        let inset = self.content_inset().min(dst_w / 2).min(dst_h / 2);
        let inner_w = dst_w.saturating_sub(inset * 2);
        let inner_h = dst_h.saturating_sub(inset * 2);
        if inner_w == 0 || inner_h == 0 {
            self.finish_draw(target, draw2d);
            return;
        }
        let (mut image_rect, mut text_rect, mut stat_rects) = self.layout(inner_w, inner_h);
        Self::offset_layout(&mut image_rect, &mut text_rect, &mut stat_rects, inset);

        if let Some(texture) = Self::explicit_profile_texture(entity, assets) {
            draw2d.blend_scale_chunk(
                self.buffer.pixels_mut(),
                &image_rect,
                stride,
                &texture.data,
                &(texture.width as usize, texture.height as usize),
            );
            self.draw_border();
            if let Some(text_rect) = text_rect {
                self.draw_identity_text(entity, assets, draw2d, text_rect);
            }
            self.draw_stat_bars(entity, draw2d, &stat_rects);
            self.finish_draw(target, draw2d);
            return;
        }

        let Some(avatar) = self
            .avatar_name
            .as_ref()
            .and_then(|name| assets.avatars.get(name))
            .or_else(|| AvatarRuntimeBuilder::find_avatar_for_entity(entity, assets))
        else {
            self.finish_draw(target, draw2d);
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
            self.finish_draw(target, draw2d);
            return;
        };

        let size = out.size as usize;
        let Some(crop_rect) = Self::crop_rect(size, &out.rgba, self.crop) else {
            self.finish_draw(target, draw2d);
            return;
        };
        let Some(cropped) = Self::cropped_rgba(&out.rgba, size, crop_rect) else {
            self.finish_draw(target, draw2d);
            return;
        };
        let (_, _, crop_w, crop_h) = crop_rect;
        draw2d.blend_scale_chunk(
            self.buffer.pixels_mut(),
            &image_rect,
            stride,
            &cropped,
            &(crop_w, crop_h),
        );
        self.draw_border();
        if let Some(text_rect) = text_rect {
            self.draw_identity_text(entity, assets, draw2d, text_rect);
        }
        self.draw_stat_bars(entity, draw2d, &stat_rects);
        self.finish_draw(target, draw2d);
    }
}
