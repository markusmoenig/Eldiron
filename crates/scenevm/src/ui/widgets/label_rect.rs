use uuid::Uuid;
use vek::Vec4;

use crate::ui::{
    drawable::Drawable,
    text::TextCache,
    workspace::{UiView, ViewContext},
};

use super::align::{HAlign, VAlign};

#[derive(Debug, Clone)]
pub struct LabelRect {
    pub id: Uuid,
    pub text: String,
    pub rect: [f32; 4], // x, y, w, h
    pub px_size: f32,
    pub color: Vec4<f32>,
    pub halign: HAlign,
    pub valign: VAlign,
    pub layer: i32,
}

impl LabelRect {
    pub fn new<T: Into<String>>(text: T, rect: [f32; 4], px_size: f32, color: Vec4<f32>) -> Self {
        Self {
            id: Uuid::new_v4(),
            text: text.into(),
            rect,
            px_size,
            color,
            halign: HAlign::Center,
            valign: VAlign::Center,
            layer: 0,
        }
    }

    pub fn with_align(mut self, halign: HAlign, valign: VAlign) -> Self {
        self.halign = halign;
        self.valign = valign;
        self
    }

    pub fn with_layer(mut self, layer: i32) -> Self {
        self.layer = layer;
        self
    }

    fn calculate_origin(&self, text_cache: &TextCache) -> [f32; 2] {
        let [rect_x, rect_y, rect_w, rect_h] = self.rect;

        // Get text dimensions
        let glyphs = text_cache.layout_positions(&self.text, self.px_size);
        let text_width = if glyphs.is_empty() {
            0.0
        } else {
            glyphs
                .iter()
                .map(|g| g.x + g.width as f32)
                .fold(0.0f32, f32::max)
        };

        // Simple height estimation: use px_size as approximate line height
        let text_height = self.px_size;

        // Calculate horizontal position
        let x = match self.halign {
            HAlign::Left => rect_x,
            HAlign::Center => rect_x + (rect_w - text_width) * 0.5,
            HAlign::Right => rect_x + rect_w - text_width,
        };

        // Calculate vertical position
        let y = match self.valign {
            VAlign::Top => rect_y,
            VAlign::Center => rect_y + (rect_h - text_height) * 0.5,
            VAlign::Bottom => rect_y + rect_h - text_height,
        };

        [x, y]
    }
}

impl UiView for LabelRect {
    fn build(&mut self, ctx: &mut ViewContext) {
        let origin = self.calculate_origin(ctx.text_cache());
        ctx.push(Drawable::Text {
            id: self.id,
            text: self.text.clone(),
            origin,
            px_size: self.px_size,
            color: self.color,
            layer: self.layer,
        });
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
