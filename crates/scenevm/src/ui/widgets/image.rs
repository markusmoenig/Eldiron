use uuid::Uuid;
use vek::Vec4;

use crate::ui::{Drawable, UiView, ViewContext};

/// Style properties for an image widget.
#[derive(Debug, Clone)]
pub struct ImageStyle {
    pub rect: [f32; 4], // x, y, w, h in pixels
    pub layer: i32,
}

/// A simple image widget that displays a texture/tile.
#[derive(Debug, Clone)]
pub struct Image {
    pub id: String,
    render_id: Uuid,
    pub style: ImageStyle,
    pub tile_id: Uuid,
    pub tint: Vec4<f32>,
}

impl Image {
    /// Create a new image widget.
    pub fn new(style: ImageStyle, tile_id: Uuid) -> Self {
        Self {
            id: String::new(),
            render_id: Uuid::new_v4(),
            style,
            tile_id,
            tint: Vec4::new(1.0, 1.0, 1.0, 1.0),
        }
    }

    /// Set the widget ID.
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Set the layer.
    pub fn with_layer(mut self, layer: i32) -> Self {
        self.style.layer = layer;
        self
    }

    /// Set the tint color (default is white/no tint).
    pub fn with_tint(mut self, tint: Vec4<f32>) -> Self {
        self.tint = tint;
        self
    }

    /// Change the tile being displayed.
    pub fn set_tile(&mut self, tile_id: Uuid) {
        self.tile_id = tile_id;
    }

    /// Change the tint color.
    pub fn set_tint(&mut self, tint: Vec4<f32>) {
        self.tint = tint;
    }
}

impl UiView for Image {
    fn build(&mut self, ctx: &mut ViewContext) {
        let [x, y, w, h] = self.style.rect;

        ctx.push(Drawable::Quad {
            id: self.render_id,
            tile_id: self.tile_id,
            rect: [x, y, w, h],
            uv: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            layer: self.style.layer,
            tint: self.tint,
        });
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn view_id(&self) -> &str {
        &self.id
    }
}
