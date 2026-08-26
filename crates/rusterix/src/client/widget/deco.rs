use crate::{
    Assets, BLACK, Currencies, Map, Pixel, Rect, Texture, WHITE,
    client::{
        draw2d,
        widget::{BorderGradientDirection, blend_texture_layer, draw_widget_border},
    },
};
use draw2d::Draw2D;
use theframework::prelude::*;

pub struct DecoWidget {
    pub rect: Rect,
    pub toml_str: String,
    pub buffer: TheRGBABuffer,
    pub layer: i32,
    pub draw2d: Draw2D,
    pub table: toml::Table,
    pub text: String,
    pub color: Pixel,
    pub border_color: Pixel,
    pub border_size: i32,
    pub border_gradient_color: Option<Pixel>,
    pub border_gradient_direction: BorderGradientDirection,
    pub border_radius: f32,
    pub border_painter: ThePainter,
    pub textures: Vec<Texture>,
    pub texture_slice: usize,
    pub separators: Vec<f32>,
    pub separator_color: Pixel,
    pub separator_gradient_color: Option<Pixel>,
    pub separator_size: i32,
    pub separator_margin: f32,
    pub top_separator_color: Pixel,
    pub top_separator_gradient_color: Option<Pixel>,
    pub top_separator_size: i32,
    pub top_separator_inset: f32,
    pub top_separator_offset: f32,
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
            layer: 0,
            draw2d: Draw2D::default(),
            table: toml::Table::default(),
            text: String::new(),
            color: BLACK,
            border_color: WHITE,
            border_size: 0,
            border_gradient_color: None,
            border_gradient_direction: BorderGradientDirection::Vertical,
            border_radius: 0.0,
            border_painter: ThePainter::new(),
            textures: vec![],
            texture_slice: 0,
            separators: vec![],
            separator_color: WHITE,
            separator_gradient_color: None,
            separator_size: 0,
            separator_margin: 0.0,
            top_separator_color: WHITE,
            top_separator_gradient_color: None,
            top_separator_size: 0,
            top_separator_inset: 0.0,
            top_separator_offset: 0.0,
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
                if let Some(value) = ui.get("layer") {
                    if let Some(v) = value.as_integer() {
                        self.layer = v as i32;
                    }
                }
                if let Some(value) = ui.get("slice") {
                    if let Some(v) = value.as_integer() {
                        self.texture_slice = v.max(0) as usize;
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
                if let Some(border) = ui.get("border").and_then(toml::Value::as_table) {
                    if let Some(v) = border.get("size").and_then(toml::Value::as_integer) {
                        self.border_size = v.max(0) as i32;
                    }
                    if let Some(v) = border
                        .get("from")
                        .or_else(|| border.get("color"))
                        .and_then(toml::Value::as_str)
                    {
                        self.border_color = self.hex_to_rgba_u8(v);
                    }
                    self.border_gradient_color = border
                        .get("to")
                        .and_then(toml::Value::as_str)
                        .map(|value| self.hex_to_rgba_u8(value));
                    self.border_gradient_direction = border
                        .get("direction")
                        .and_then(toml::Value::as_str)
                        .map(BorderGradientDirection::parse)
                        .unwrap_or_default();
                    self.border_radius = border
                        .get("radius")
                        .and_then(|value| {
                            value
                                .as_float()
                                .or_else(|| value.as_integer().map(|v| v as f64))
                        })
                        .unwrap_or(0.0) as f32;
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

        if let Some(texture) = self.textures.first() {
            blend_texture_layer(buffer, self.rect, &self.draw2d, texture, self.texture_slice);
        }

        if self.separator_size > 0 && !self.separators.is_empty() {
            let surface_width = buffer.dim().width.max(0) as usize;
            let surface_height = buffer.dim().height.max(0) as usize;
            if let Ok(mut surface) =
                TheSurfaceMut::new(buffer.pixels_mut(), surface_width, surface_height)
            {
                let top = self.rect.y + self.separator_margin;
                let height = (self.rect.height - self.separator_margin * 2.0).max(0.0);
                if height > 0.0 {
                    for separator in &self.separators {
                        let paint = self
                            .separator_gradient_color
                            .map(|end_color| {
                                ThePaint::linear_gradient(
                                    [*separator, top],
                                    [*separator, top + height],
                                    self.separator_color,
                                    end_color,
                                )
                            })
                            .unwrap_or_else(|| ThePaint::solid(self.separator_color));
                        self.border_painter.fill_rect(
                            &mut surface,
                            ThePixelRect::new(
                                (*separator - self.separator_size as f32 * 0.5).round() as i32,
                                top.round() as i32,
                                self.separator_size,
                                height.round() as i32,
                            ),
                            &paint,
                        );
                    }
                }
            }
        }

        if self.top_separator_size > 0 {
            let surface_width = buffer.dim().width.max(0) as usize;
            let surface_height = buffer.dim().height.max(0) as usize;
            if let Ok(mut surface) =
                TheSurfaceMut::new(buffer.pixels_mut(), surface_width, surface_height)
            {
                let left = self.rect.x + self.top_separator_inset;
                let top = self.rect.y + self.top_separator_offset;
                let width = (self.rect.width - self.top_separator_inset * 2.0).max(0.0);
                if width > 0.0 {
                    let paint = self
                        .top_separator_gradient_color
                        .map(|end_color| {
                            ThePaint::linear_gradient(
                                [left, top],
                                [left + width, top],
                                self.top_separator_color,
                                end_color,
                            )
                        })
                        .unwrap_or_else(|| ThePaint::solid(self.top_separator_color));
                    self.border_painter.fill_rect(
                        &mut surface,
                        ThePixelRect::new(
                            left.round() as i32,
                            top.round() as i32,
                            width.round() as i32,
                            self.top_separator_size,
                        ),
                        &paint,
                    );
                }
            }
        }

        if self.border_size > 0 {
            draw_widget_border(
                buffer,
                self.rect,
                &self.draw2d,
                &mut self.border_painter,
                self.border_size,
                self.border_color,
                self.border_gradient_color,
                self.border_gradient_direction,
                self.border_radius,
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
