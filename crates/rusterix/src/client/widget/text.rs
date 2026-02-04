use crate::{Assets, Currencies, Map, Pixel, Rect, WHITE, client::draw2d};
use draw2d::Draw2D;
use regex::Regex;
use theframework::prelude::*;

fn substitute_placeholders<F>(input: &str, mut resolver: F) -> String
where
    F: FnMut(&str, &str) -> Option<String>,
{
    let re = Regex::new(r"\{([A-Z_]+)\.([A-Z0-9_]+)\}").unwrap();

    re.replace_all(input, |caps: &regex::Captures| {
        let category = &caps[1];
        let key = &caps[2];
        resolver(category, key).unwrap_or_else(|| format!("{{{}.{}?}}", category, key))
    })
    .to_string()
}

pub struct TextWidget {
    pub name: String,
    pub rect: Rect,
    pub toml_str: String,
    pub buffer: TheRGBABuffer,
    pub font: Option<fontdue::Font>,
    pub font_size: f32,
    pub messages: Vec<(String, Pixel)>,
    pub draw2d: Draw2D,
    pub spacing: f32,
    pub table: toml::Table,
    pub text: String,
    pub color: Pixel,
}

impl Default for TextWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl TextWidget {
    pub fn new() -> Self {
        Self {
            name: String::new(),
            rect: Rect::default(),
            toml_str: String::new(),
            buffer: TheRGBABuffer::default(),
            font: None,
            font_size: 20.0,
            messages: vec![],
            draw2d: Draw2D::default(),
            spacing: 1.0,
            table: toml::Table::default(),
            text: String::new(),
            color: WHITE,
        }
    }

    pub fn init(&mut self, assets: &Assets) {
        let mut font_name = String::new();
        if let Ok(table) = self.toml_str.parse::<toml::Table>() {
            if let Some(ui) = table.get("ui").and_then(toml::Value::as_table) {
                if let Some(value) = ui.get("font") {
                    if let Some(v) = value.as_str() {
                        font_name = v.into();
                    }
                }
                if let Some(value) = ui.get("font_size") {
                    if let Some(v) = value.as_float() {
                        self.font_size = v as f32;
                    }
                }
                if let Some(value) = ui.get("spacing") {
                    if let Some(v) = value.as_float() {
                        self.spacing = v as f32;
                    }
                }
                if let Some(value) = ui.get("text") {
                    if let Some(v) = value.as_str() {
                        self.text = v.into();
                    }
                }
                if let Some(value) = ui.get("color") {
                    if let Some(v) = value.as_str() {
                        self.color = self.hex_to_rgba_u8(v);
                    }
                }
            }
            self.table = table;
        }

        if let Some(font) = assets.fonts.get(&font_name) {
            self.font = Some(font.clone());
        }
    }

    pub fn update_draw(
        &mut self,
        buffer: &mut TheRGBABuffer,
        map: &Map,
        currencies: &Currencies,
        _assets: &Assets,
    ) {
        if let Some(font) = &self.font {
            let stride = buffer.stride();
            let mut y = self.rect.y;

            let width = buffer.dim().width;
            let height = buffer.dim().height;

            for line in self.text.lines() {
                let resolved = substitute_placeholders(line, |cat, key| {
                    match cat {
                        "PLAYER" => {
                            for entity in &map.entities {
                                if entity.is_player() {
                                    if key == "FUNDS" {
                                        return Some(
                                            entity.wallet.get_balance(currencies).to_string(),
                                        );
                                    } else if let Some(value) = entity.attributes.get(key) {
                                        return Some(value.to_string());
                                    }
                                }
                            }
                            None
                        }
                        // "WORLD" => map.world.get_value(key),
                        _ => None,
                    }
                });

                let tuple = (
                    self.rect.x as isize,
                    y.floor() as isize,
                    self.rect.width as isize,
                    self.font_size as isize,
                );

                self.draw2d.text_rect_blend_safe(
                    buffer.pixels_mut(),
                    &tuple,
                    stride,
                    font,
                    self.font_size,
                    &resolved,
                    &self.color,
                    draw2d::TheHorizontalAlign::Left,
                    draw2d::TheVerticalAlign::Center,
                    &(0, 0, width as isize, height as isize),
                );

                y += self.font_size + self.spacing;
            }
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
