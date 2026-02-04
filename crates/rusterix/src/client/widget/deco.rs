use crate::{Assets, BLACK, Currencies, Map, Pixel, Rect, WHITE, client::draw2d};
use draw2d::Draw2D;
use theframework::prelude::*;

pub struct DecoWidget {
    pub rect: Rect,
    pub toml_str: String,
    pub buffer: TheRGBABuffer,
    pub draw2d: Draw2D,
    pub table: toml::Table,
    pub text: String,
    pub color: Pixel,
    pub border_color: Pixel,
    pub border_size: i32,
}

impl Default for DecoWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl DecoWidget {
    pub fn new() -> Self {
        Self {
            rect: Rect::default(),
            toml_str: String::new(),
            buffer: TheRGBABuffer::default(),
            draw2d: Draw2D::default(),
            table: toml::Table::default(),
            text: String::new(),
            color: BLACK,
            border_color: WHITE,
            border_size: 1,
        }
    }

    pub fn init(&mut self, _assets: &Assets) {
        if let Ok(table) = self.toml_str.parse::<toml::Table>() {
            if let Some(ui) = table.get("ui").and_then(toml::Value::as_table) {
                if let Some(value) = ui.get("border_size") {
                    if let Some(v) = value.as_integer() {
                        self.border_size = v as i32;
                    }
                }
                if let Some(value) = ui.get("color") {
                    if let Some(v) = value.as_str() {
                        self.color = self.hex_to_rgba_u8(v);
                    }
                }
                if let Some(value) = ui.get("border_color") {
                    if let Some(v) = value.as_str() {
                        self.border_color = self.hex_to_rgba_u8(v);
                    }
                }
            }
            self.table = table;
        }
    }

    pub fn update_draw(
        &mut self,
        buffer: &mut TheRGBABuffer,
        _map: &Map,
        _currencies: &Currencies,
        _assets: &Assets,
    ) {
        let stride = buffer.stride();

        self.draw2d.blend_rect(
            buffer.pixels_mut(),
            &(
                self.rect.x as usize,
                self.rect.y as usize,
                self.rect.width as usize,
                self.rect.height as usize,
            ),
            stride,
            &self.color,
        );

        if self.border_size > 0 {
            self.draw2d.rect_outline_thickness(
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

    /// Converts a hex color string to a [u8; 4] (RGBA).
    /// Accepts "#RRGGBB" or "#RRGGBBAA" formats.
    fn hex_to_rgba_u8(&self, hex: &str) -> [u8; 4] {
        let hex = hex.trim_start_matches('#');

        match hex.len() {
            6 => match (
                u8::from_str_radix(&hex[0..2], 16),
                u8::from_str_radix(&hex[2..4], 16),
                u8::from_str_radix(&hex[4..6], 16),
            ) {
                (Ok(r), Ok(g), Ok(b)) => [r, g, b, 255],
                _ => [255, 255, 255, 255],
            },
            8 => match (
                u8::from_str_radix(&hex[0..2], 16),
                u8::from_str_radix(&hex[2..4], 16),
                u8::from_str_radix(&hex[4..6], 16),
                u8::from_str_radix(&hex[6..8], 16),
            ) {
                (Ok(r), Ok(g), Ok(b), Ok(a)) => [r, g, b, a],
                _ => [255, 255, 255, 255],
            },
            _ => [255, 255, 255, 255],
        }
    }
}
