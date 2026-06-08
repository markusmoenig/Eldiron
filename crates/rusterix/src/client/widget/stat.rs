use crate::{Assets, Entity, Pixel, Rect, client::draw2d};
use draw2d::Draw2D;
use theframework::prelude::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StatWidgetMode {
    Bar,
    Tiles,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StatWidgetOrientation {
    Horizontal,
    Vertical,
}

pub struct StatWidget {
    pub name: String,
    pub rect: Rect,
    pub toml_str: String,
    pub buffer: TheRGBABuffer,
    pub party: Option<String>,

    stat: String,
    max_stat: String,
    mode: StatWidgetMode,
    orientation: StatWidgetOrientation,
    tile_ids: Vec<Uuid>,
    clip_tile: bool,
    background_color: Pixel,
    fill_color: Pixel,
    border_color: Pixel,
    border_size: i32,
}

impl Default for StatWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl StatWidget {
    pub fn new() -> Self {
        Self {
            name: String::new(),
            rect: Rect::default(),
            toml_str: String::new(),
            buffer: TheRGBABuffer::default(),
            party: None,
            stat: "HP".into(),
            max_stat: "MAX_HP".into(),
            mode: StatWidgetMode::Bar,
            orientation: StatWidgetOrientation::Horizontal,
            tile_ids: Vec::new(),
            clip_tile: true,
            background_color: [0, 0, 0, 0],
            fill_color: [196, 48, 48, 255],
            border_color: [255, 255, 255, 255],
            border_size: 0,
        }
    }

    pub fn init(&mut self) {
        if let Ok(value) = self.toml_str.parse::<toml::Table>()
            && let Some(ui) = value.get("ui").and_then(toml::Value::as_table)
        {
            if let Some(v) = ui
                .get("party")
                .and_then(toml::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                self.party = Some(v.to_string());
            }

            if let Some(v) = ui
                .get("stat")
                .or_else(|| ui.get("attribute"))
                .or_else(|| ui.get("value"))
                .and_then(toml::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                self.stat = v.to_string();
            }

            self.max_stat = ui
                .get("max_stat")
                .or_else(|| ui.get("max_attribute"))
                .or_else(|| ui.get("max"))
                .and_then(toml::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| format!("MAX_{}", self.stat));

            if let Some(v) = ui.get("mode").and_then(toml::Value::as_str) {
                self.mode = match v.trim().to_ascii_lowercase().as_str() {
                    "tile" | "tiles" | "icon" | "icons" => StatWidgetMode::Tiles,
                    _ => StatWidgetMode::Bar,
                };
            }

            if let Some(v) = ui
                .get("orientation")
                .or_else(|| ui.get("direction"))
                .and_then(toml::Value::as_str)
            {
                self.orientation = match v.trim().to_ascii_lowercase().as_str() {
                    "vertical" | "up" | "down" => StatWidgetOrientation::Vertical,
                    _ => StatWidgetOrientation::Horizontal,
                };
            }
            if ui
                .get("vertical")
                .and_then(toml::Value::as_bool)
                .unwrap_or(false)
            {
                self.orientation = StatWidgetOrientation::Vertical;
            }

            self.tile_ids = Self::parse_tile_ids(ui);
            if !self.tile_ids.is_empty() && ui.get("mode").is_none() {
                self.mode = StatWidgetMode::Tiles;
                self.orientation = StatWidgetOrientation::Vertical;
            }
            if let Some(v) = ui.get("clip").and_then(toml::Value::as_bool) {
                self.clip_tile = v;
            }

            if let Some(v) = ui
                .get("background_color")
                .or_else(|| ui.get("background"))
                .and_then(toml::Value::as_str)
            {
                self.background_color = Self::hex_to_rgba_u8(v);
            }
            if let Some(v) = ui
                .get("fill_color")
                .or_else(|| ui.get("color"))
                .and_then(toml::Value::as_str)
            {
                self.fill_color = Self::hex_to_rgba_u8(v);
            }
            if let Some(v) = ui.get("border_color").and_then(toml::Value::as_str) {
                self.border_color = Self::hex_to_rgba_u8(v);
            }
            if let Some(v) = ui.get("border_size").and_then(toml::Value::as_integer) {
                self.border_size = v as i32;
            }
        }
    }

    pub fn update_draw(
        &mut self,
        target: &mut TheRGBABuffer,
        assets: &Assets,
        entity: Option<&Entity>,
        draw2d: &Draw2D,
        animation_frame: usize,
    ) {
        self.buffer.fill([0, 0, 0, 0]);

        let ratio = entity
            .map(|entity| {
                let current = entity.attributes.get_float_default(&self.stat, 0.0);
                let max = entity
                    .attributes
                    .get_float_default(&self.max_stat, current.max(1.0));
                if max <= 0.0 {
                    0.0
                } else {
                    (current / max).clamp(0.0, 1.0)
                }
            })
            .unwrap_or(0.0);

        if self.background_color[3] > 0 {
            let width = self.buffer.dim().width as usize;
            let height = self.buffer.dim().height as usize;
            let stride = self.buffer.stride();
            draw2d.blend_rect(
                self.buffer.pixels_mut(),
                &(0, 0, width, height),
                stride,
                &self.background_color,
            );
        }

        match self.mode {
            StatWidgetMode::Tiles => {
                if self.tile_ids.len() == 1 && self.clip_tile {
                    self.draw_clipped_source(assets, draw2d, animation_frame, ratio);
                } else if let Some(source_id) = self.source_for_ratio(ratio) {
                    Self::draw_source_to_buffer(
                        &mut self.buffer,
                        assets,
                        draw2d,
                        animation_frame,
                        source_id,
                    );
                }
            }
            StatWidgetMode::Bar => self.draw_bar(draw2d, ratio),
        }

        if self.border_size > 0 {
            let width = self.buffer.dim().width as usize;
            let height = self.buffer.dim().height as usize;
            let stride = self.buffer.stride();
            draw2d.rect_outline_thickness(
                self.buffer.pixels_mut(),
                &(0, 0, width, height),
                stride,
                &self.border_color,
                self.border_size as usize,
            );
        }

        target.blend_into(self.rect.x as i32, self.rect.y as i32, &self.buffer);
    }

    fn parse_tile_ids(ui: &toml::Table) -> Vec<Uuid> {
        if let Some(values) = ui.get("tile_ids").and_then(toml::Value::as_array) {
            return values
                .iter()
                .filter_map(toml::Value::as_str)
                .filter_map(|value| Uuid::parse_str(value.trim()).ok())
                .collect();
        }

        ui.get("tile_id")
            .and_then(toml::Value::as_str)
            .and_then(|value| Uuid::parse_str(value.trim()).ok())
            .into_iter()
            .collect()
    }

    fn source_for_ratio(&self, ratio: f32) -> Option<Uuid> {
        if self.tile_ids.is_empty() {
            return None;
        }
        let last = self.tile_ids.len().saturating_sub(1);
        let index = (ratio.clamp(0.0, 1.0) * last as f32).round() as usize;
        self.tile_ids.get(index.min(last)).copied()
    }

    fn draw_bar(&mut self, draw2d: &Draw2D, ratio: f32) {
        let width = self.buffer.dim().width.max(0) as usize;
        let height = self.buffer.dim().height.max(0) as usize;
        if width == 0 || height == 0 || ratio <= 0.0 || self.fill_color[3] == 0 {
            return;
        }

        match self.orientation {
            StatWidgetOrientation::Horizontal => {
                let fill_width = ((width as f32) * ratio).round().clamp(0.0, width as f32) as usize;
                let stride = self.buffer.stride();
                draw2d.blend_rect(
                    self.buffer.pixels_mut(),
                    &(0, 0, fill_width, height),
                    stride,
                    &self.fill_color,
                );
            }
            StatWidgetOrientation::Vertical => {
                let fill_height =
                    ((height as f32) * ratio).round().clamp(0.0, height as f32) as usize;
                let y = height.saturating_sub(fill_height);
                let stride = self.buffer.stride();
                draw2d.blend_rect(
                    self.buffer.pixels_mut(),
                    &(0, y, width, fill_height),
                    stride,
                    &self.fill_color,
                );
            }
        }
    }

    fn draw_clipped_source(
        &mut self,
        assets: &Assets,
        draw2d: &Draw2D,
        animation_frame: usize,
        ratio: f32,
    ) {
        let Some(source_id) = self.tile_ids.first().copied() else {
            return;
        };
        let dst_w = self.buffer.dim().width.max(0) as usize;
        let dst_h = self.buffer.dim().height.max(0) as usize;
        if dst_w == 0 || dst_h == 0 || ratio <= 0.0 {
            return;
        }
        let visible_w = match self.orientation {
            StatWidgetOrientation::Horizontal => {
                ((dst_w as f32) * ratio).round().clamp(0.0, dst_w as f32) as usize
            }
            StatWidgetOrientation::Vertical => dst_w,
        };
        let visible_h = match self.orientation {
            StatWidgetOrientation::Horizontal => dst_h,
            StatWidgetOrientation::Vertical => {
                ((dst_h as f32) * ratio).round().clamp(0.0, dst_h as f32) as usize
            }
        };
        let offset_y = if self.orientation == StatWidgetOrientation::Vertical {
            dst_h.saturating_sub(visible_h)
        } else {
            0
        };

        let mut scaled = TheRGBABuffer::new(TheDim::sized(dst_w as i32, dst_h as i32));
        scaled.fill([0, 0, 0, 0]);
        Self::draw_source_to_buffer(&mut scaled, assets, draw2d, animation_frame, source_id);

        let src = scaled.pixels();
        let stride = self.buffer.stride();
        let dst = self.buffer.pixels_mut();
        for y in offset_y..offset_y + visible_h {
            let row = y * stride * 4;
            for x in 0..visible_w {
                let index = row + x * 4;
                if index + 3 >= src.len() || index + 3 >= dst.len() {
                    continue;
                }
                let alpha = src[index + 3];
                if alpha == 0 {
                    continue;
                }
                let background = [dst[index], dst[index + 1], dst[index + 2], dst[index + 3]];
                let source = [src[index], src[index + 1], src[index + 2], alpha];
                dst[index..index + 4].copy_from_slice(&draw2d.mix_color(
                    &background,
                    &source,
                    alpha as f32 / 255.0,
                ));
            }
        }
    }

    fn draw_source_to_buffer(
        buffer: &mut TheRGBABuffer,
        assets: &Assets,
        draw2d: &Draw2D,
        animation_frame: usize,
        source_id: Uuid,
    ) {
        let dst_w = buffer.dim().width.max(0) as usize;
        let dst_h = buffer.dim().height.max(0) as usize;
        if dst_w == 0 || dst_h == 0 {
            return;
        }

        if let Some(tile) = assets.tiles.get(&source_id) {
            if tile.textures.is_empty() {
                return;
            }
            let texture = &tile.textures[animation_frame % tile.textures.len()];
            let stride = buffer.stride();
            draw2d.blend_scale_chunk(
                buffer.pixels_mut(),
                &(0, 0, dst_w, dst_h),
                stride,
                &texture.data,
                &(texture.width, texture.height),
            );
            return;
        }

        let Some(group) = assets.tile_groups.get(&source_id) else {
            return;
        };
        let group_w = group.width.max(1) as usize;
        let group_h = group.height.max(1) as usize;
        let stride = buffer.stride();
        for member in &group.members {
            let Some(tile) = assets.tiles.get(&member.tile_id) else {
                continue;
            };
            if tile.textures.is_empty() {
                continue;
            }
            let texture = &tile.textures[animation_frame % tile.textures.len()];
            let x0 = member.x as usize * dst_w / group_w;
            let y0 = member.y as usize * dst_h / group_h;
            let x1 = ((member.x as usize + 1) * dst_w / group_w).max(x0 + 1);
            let y1 = ((member.y as usize + 1) * dst_h / group_h).max(y0 + 1);
            let width = x1.saturating_sub(x0).min(dst_w.saturating_sub(x0));
            let height = y1.saturating_sub(y0).min(dst_h.saturating_sub(y0));
            if width == 0 || height == 0 {
                continue;
            }
            draw2d.blend_scale_chunk(
                buffer.pixels_mut(),
                &(x0, y0, width, height),
                stride,
                &texture.data,
                &(texture.width, texture.height),
            );
        }
    }

    fn hex_to_rgba_u8(hex: &str) -> Pixel {
        let hex = hex.trim().trim_start_matches('#');
        if !(hex.len() == 6 || hex.len() == 8) {
            return [255, 255, 255, 255];
        }
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(255);
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(255);
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(255);
        let a = if hex.len() == 8 {
            u8::from_str_radix(&hex[6..8], 16).unwrap_or(255)
        } else {
            255
        };
        [r, g, b, a]
    }
}
