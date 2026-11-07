//! Live screen artwork references. Visual states, animation and spatial pages are independent.
use super::{ButtonVisualState, Widget, blend_texture_layer};
use crate::{Assets, PixelSource, Rect, RepeatMode, SampleMode};
use theframework::prelude::*;

#[derive(Clone, Default)]
pub struct ScreenArtwork {
    pub sources: Vec<PixelSource>,
    pub repeat: bool,
    pub slice: usize,
    /// Authored screen-grid coordinates, shared with screen surface baking.
    pub origin: [f32; 2],
    pub units_per_pixel: f32,
}

impl ScreenArtwork {
    pub fn draw(
        &self,
        target: &mut TheRGBABuffer,
        rect: Rect,
        assets: &Assets,
        state: ButtonVisualState,
        frame: usize,
        draw: &crate::client::draw2d::Draw2D,
    ) -> bool {
        if self.sources.is_empty() {
            return false;
        }
        let source = &self.sources[Widget::texture_index_for_state(self.sources.len(), state)];
        let id = source.render_tile_id(assets);
        let Some(tile) = id
            .as_ref()
            .and_then(|id| assets.tiles.get(id).or_else(|| assets.materials.get(id)))
        else {
            return true;
        };
        if !self.repeat {
            if let Some(texture) = tile.textures.get(tile.frame_index(frame)) {
                blend_texture_layer(target, rect, draw, texture, self.slice);
            }
            return true;
        }
        let width = target.dim().width.max(0) as usize;
        let height = target.dim().height.max(0) as usize;
        let x0 = (rect.x.floor().max(0.) as usize).min(width);
        let y0 = (rect.y.floor().max(0.) as usize).min(height);
        let x1 = ((rect.x + rect.width).ceil().max(0.) as usize).min(width);
        let y1 = ((rect.y + rect.height).ceil().max(0.) as usize).min(height);
        let pixels = target.pixels_mut();
        for y in y0..y1 {
            for x in x0..x1 {
                let uv = [
                    self.origin[0] + (x as f32 + 0.5 - rect.x) * self.units_per_pixel,
                    self.origin[1] + (y as f32 + 0.5 - rect.y) * self.units_per_pixel,
                ];
                let color = tile.sample_2d(uv, frame, SampleMode::Nearest, RepeatMode::RepeatXY);
                let i = (y * width + x) * 4;
                let mixed = theframework::thergbabuffer::source_over(
                    pixels[i..i + 4].try_into().unwrap(),
                    color,
                    color[3] as f32 / 255.,
                );
                pixels[i..i + 4].copy_from_slice(&mixed);
            }
        }
        true
    }
}
