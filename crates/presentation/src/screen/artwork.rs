use super::*;
use rusterix::{
    Assets, Rect, Texture,
    client::widget::{
        BorderGradientDirection, ButtonVisualState, Widget, artwork::ScreenArtwork,
        deco::DecoWidget,
    },
};
use std::sync::Arc;
use theframework::paint::{
    Brush, LineCap, LineJoin, PaintOp, Path, PathElement as E, Rect as Uv, Sampling, Stroke,
};

pub(super) fn segments(
    start: f32,
    length: f32,
    origin: i32,
    count: u32,
    units: f32,
) -> Vec<(f32, f32, usize)> {
    let end = start + length;
    let mut boundaries = vec![start];
    for i in 0..=count {
        let value = (origin as f32 + i as f32) * units;
        if value > start && value < end {
            boundaries.push(value);
        }
    }
    boundaries.push(end);
    boundaries
        .windows(2)
        .map(|b| {
            let page = (((b[0] + b[1]) * 0.5 / units).floor() - origin as f32)
                .clamp(0., count.saturating_sub(1) as f32) as usize;
            (b[0], b[1], page)
        })
        .collect()
}

impl ScreenPainter<'_> {
    /// Nine source rectangles preserve authored edges without resizing a CPU image.
    pub fn sliced_texture(
        &mut self,
        key: &str,
        texture: &Texture,
        bounds: Rect,
        slice: usize,
    ) -> Result<(), String> {
        let sx = slice.min(texture.width.saturating_sub(1) / 2) as f32;
        let sy = slice.min(texture.height.saturating_sub(1) / 2) as f32;
        if sx == 0. || sy == 0. {
            return self.texture(
                key,
                texture,
                bounds,
                Uv::new(0., 0., 1., 1.),
                Sampling::Nearest,
                [1.; 4],
            );
        }
        let dx = sx.min(bounds.width * 0.5);
        let dy = sy.min(bounds.height * 0.5);
        let x = [
            bounds.x,
            bounds.x + dx,
            bounds.x + bounds.width - dx,
            bounds.x + bounds.width,
        ];
        let y = [
            bounds.y,
            bounds.y + dy,
            bounds.y + bounds.height - dy,
            bounds.y + bounds.height,
        ];
        let u = [
            0.,
            sx / texture.width as f32,
            1. - sx / texture.width as f32,
            1.,
        ];
        let v = [
            0.,
            sy / texture.height as f32,
            1. - sy / texture.height as f32,
            1.,
        ];
        let image = self.images.rgba(
            key,
            texture.width as u32,
            texture.height as u32,
            &texture.data,
        )?;
        for j in 0..3 {
            for i in 0..3 {
                let bounds = Rect::new(x[i], y[j], x[i + 1] - x[i], y[j + 1] - y[j]);
                if bounds.width <= 0. || bounds.height <= 0. {
                    continue;
                }
                self.list.ops.push(PaintOp::Image {
                    image,
                    rect: rect(bounds),
                    uv: Uv::new(u[i], v[j], u[i + 1] - u[i], v[j + 1] - v[j]),
                    sampling: Sampling::Nearest,
                    tint: [1.; 4],
                });
            }
        }
        Ok(())
    }
    /// Return true for an authored reference even if its asset is currently absent,
    /// preserving the controller's authored-artwork precedence.
    pub fn artwork(
        &mut self,
        artwork: &ScreenArtwork,
        bounds: Rect,
        assets: &Assets,
        state: ButtonVisualState,
        frame: usize,
    ) -> Result<bool, String> {
        if artwork.sources.is_empty() {
            return Ok(false);
        }
        let source =
            &artwork.sources[Widget::texture_index_for_state(artwork.sources.len(), state)];
        let id = source.render_tile_id(assets);
        let Some(tile) =
            id.and_then(|id| assets.tiles.get(&id).or_else(|| assets.materials.get(&id)))
        else {
            return Ok(true);
        };
        let phase = tile.frame_index(frame);
        if !artwork.repeat {
            if let Some(texture) = tile.textures.get(phase) {
                self.sliced_texture(
                    &format!("tile/{}/{phase}", tile.id),
                    texture,
                    bounds,
                    artwork.slice,
                )?;
            }
            return Ok(true);
        }
        let density = artwork.units_per_pixel;
        if density <= 0. || !density.is_finite() || bounds.width <= 0. || bounds.height <= 0. {
            return Ok(true);
        }
        if let Some(pages) = &tile.uv_pages {
            if pages.repeat_period.iter().all(|v| v.is_finite() && *v > 0.) {
                if let Some(texture) = tile.textures.get(phase) {
                    self.texture(
                        &format!("tile/{}/{phase}", tile.id),
                        texture,
                        bounds,
                        Uv::new(
                            artwork.origin[0] / pages.repeat_period[0],
                            artwork.origin[1] / pages.repeat_period[1],
                            bounds.width * density / pages.repeat_period[0],
                            bounds.height * density / pages.repeat_period[1],
                        ),
                        Sampling::RepeatNearest,
                        [1.; 4],
                    )?;
                }
            } else if pages.size.iter().all(|n| *n > 0) {
                let units = pages.units_per_page.max(1) as f32;
                let xs = segments(
                    artwork.origin[0],
                    bounds.width * density,
                    pages.origin[0],
                    pages.size[0],
                    units,
                );
                let ys = segments(
                    artwork.origin[1],
                    bounds.height * density,
                    pages.origin[1],
                    pages.size[1],
                    units,
                );
                for &(y0, y1, j) in &ys {
                    for &(x0, x1, i) in &xs {
                        let index = phase + j * pages.size[0] as usize + i;
                        if let Some(texture) = tile.textures.get(index) {
                            self.texture(
                                &format!("tile/{}/{index}", tile.id),
                                texture,
                                Rect::new(
                                    bounds.x + (x0 - artwork.origin[0]) / density,
                                    bounds.y + (y0 - artwork.origin[1]) / density,
                                    (x1 - x0) / density,
                                    (y1 - y0) / density,
                                ),
                                Uv::new(
                                    x0 / units,
                                    y0 / units,
                                    (x1 - x0) / units,
                                    (y1 - y0) / units,
                                ),
                                Sampling::RepeatNearest,
                                [1.; 4],
                            )?;
                        }
                    }
                }
            }
        } else if let Some(texture) = tile.textures.get(phase) {
            self.texture(
                &format!("tile/{}/{phase}", tile.id),
                texture,
                bounds,
                Uv::new(
                    artwork.origin[0],
                    artwork.origin[1],
                    bounds.width * density,
                    bounds.height * density,
                ),
                Sampling::RepeatNearest,
                [1.; 4],
            )?;
        }
        Ok(true)
    }
    pub fn gradient_rect(
        &mut self,
        r: Rect,
        start: rusterix::Pixel,
        end: Option<rusterix::Pixel>,
        vertical: bool,
    ) {
        let brush = end.map_or_else(
            || Brush::Solid(color(start)),
            |end| Brush::LinearGradient {
                start: [r.x, r.y],
                end: if vertical {
                    [r.x, r.y + r.height]
                } else {
                    [r.x + r.width, r.y]
                },
                start_color: color(start),
                end_color: color(end),
            },
        );
        if r.width > 0. && r.height > 0. {
            self.list.rect(rect(r), 0., brush);
        }
    }
    pub fn styled_border(
        &mut self,
        bounds: Rect,
        width: f32,
        start: rusterix::Pixel,
        end: Option<rusterix::Pixel>,
        direction: BorderGradientDirection,
        radius: f32,
    ) {
        let width = width.max(0.).min(bounds.width).min(bounds.height);
        if width <= 0. {
            return;
        }
        let inset = width * 0.5;
        let r = Rect::new(
            bounds.x + inset,
            bounds.y + inset,
            bounds.width - width,
            bounds.height - width,
        );
        if r.width <= 0. || r.height <= 0. {
            self.fill(bounds, start);
            return;
        }
        let radius = radius.max(0.).min(r.width.min(r.height) * 0.5);
        let (x, y, w, h, q) = (r.x, r.y, r.width, r.height, radius);
        let k = 0.5522848 * q;
        let path = Path {
            elements: vec![
                E::Move([x + q, y]),
                E::Line([x + w - q, y]),
                E::Cubic([x + w - q + k, y], [x + w, y + q - k], [x + w, y + q]),
                E::Line([x + w, y + h - q]),
                E::Cubic(
                    [x + w, y + h - q + k],
                    [x + w - q + k, y + h],
                    [x + w - q, y + h],
                ),
                E::Line([x + q, y + h]),
                E::Cubic([x + q - k, y + h], [x, y + h - q + k], [x, y + h - q]),
                E::Line([x, y + q]),
                E::Cubic([x, y + q - k], [x + q - k, y], [x + q, y]),
                E::Close,
            ],
        };
        let brush = end.map_or_else(
            || Brush::Solid(color(start)),
            |end| {
                let (a, b) = match direction {
                    BorderGradientDirection::Horizontal => ([x, y], [x + w, y]),
                    BorderGradientDirection::Vertical => ([x, y], [x, y + h]),
                    BorderGradientDirection::Diagonal => ([x, y], [x + w, y + h]),
                    BorderGradientDirection::DiagonalReverse => ([x + w, y], [x, y + h]),
                };
                Brush::LinearGradient {
                    start: a,
                    end: b,
                    start_color: color(start),
                    end_color: color(end),
                }
            },
        );
        self.list.ops.push(PaintOp::Stroke {
            path: Arc::new(path),
            stroke: Stroke {
                width,
                cap: LineCap::Butt,
                join: LineJoin::Round,
                brush,
            },
        });
    }
    pub fn decoration(&mut self, widget: &DecoWidget, assets: &Assets) -> Result<(), String> {
        self.fill(widget.rect, widget.color);
        if !self.artwork(
            &widget.artwork,
            widget.rect,
            assets,
            ButtonVisualState::Normal,
            widget.animation_frame,
        )? {
            if let Some(texture) = widget.textures.first() {
                self.sliced_texture(
                    &format!("deco/{:p}", texture),
                    texture,
                    widget.rect,
                    widget.texture_slice,
                )?;
            }
        }
        if widget.separator_size > 0 {
            for x in &widget.separators {
                self.gradient_rect(
                    Rect::new(
                        *x - widget.separator_size as f32 * 0.5,
                        widget.rect.y + widget.separator_margin,
                        widget.separator_size as f32,
                        (widget.rect.height - widget.separator_margin * 2.).max(0.),
                    ),
                    widget.separator_color,
                    widget.separator_gradient_color,
                    true,
                );
            }
        }
        self.gradient_rect(
            Rect::new(
                widget.rect.x + widget.top_separator_inset,
                widget.rect.y + widget.top_separator_offset,
                (widget.rect.width - widget.top_separator_inset * 2.).max(0.),
                widget.top_separator_size.max(0) as f32,
            ),
            widget.top_separator_color,
            widget.top_separator_gradient_color,
            false,
        );
        self.styled_border(
            widget.rect,
            widget.border_size.max(0) as f32,
            widget.border_color,
            widget.border_gradient_color,
            widget.border_gradient_direction,
            widget.border_radius,
        );
        Ok(())
    }
}
