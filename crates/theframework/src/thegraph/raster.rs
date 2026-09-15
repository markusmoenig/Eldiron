use super::*;
use crate::prelude::*;
use fontdue::{Font, Metrics};
use std::collections::HashMap;

/// Asset lookup stays with the host. Missing previews receive a neutral placeholder.
pub trait GraphAssets {
    fn image(&self, key: &str) -> Option<&TheRGBABuffer>;
}
impl GraphAssets for () {
    fn image(&self, _: &str) -> Option<&TheRGBABuffer> {
        None
    }
}

/// Reusable raster resources. Glyphs are rasterized at display density and graph zoom.
pub struct GraphRasterResources {
    pub font: Font,
    painter: ThePainter,
    glyphs: HashMap<(char, u32), (Metrics, Vec<u8>)>,
}
impl GraphRasterResources {
    pub fn new(font: Font) -> Self {
        Self {
            font,
            painter: ThePainter::new(),
            glyphs: HashMap::new(),
        }
    }
}
/// Clips every primitive to the framebuffer. The host can supply a local framebuffer
/// for embedding in a panel and composite it using its normal UI machinery.
pub struct RasterGraphPainter<'a> {
    pixels: &'a mut [u8],
    width: usize,
    height: usize,
    density: f32,
    resources: &'a mut GraphRasterResources,
    assets: &'a dyn GraphAssets,
}
impl<'a> RasterGraphPainter<'a> {
    pub fn new(
        pixels: &'a mut [u8],
        width: usize,
        height: usize,
        density: f32,
        resources: &'a mut GraphRasterResources,
        assets: &'a dyn GraphAssets,
    ) -> Self {
        assert!(
            width
                .checked_mul(height)
                .and_then(|n| n.checked_mul(4))
                .is_some_and(|n| pixels.len() >= n)
        );
        assert!(density.is_finite() && density > 0.);
        Self {
            pixels,
            width,
            height,
            density,
            resources,
            assets,
        }
    }
    fn blend(&mut self, x: i32, y: i32, c: GraphColor, alpha: f32) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }
        let i = (y as usize * self.width + x as usize) * 4;
        for k in 0..3 {
            self.pixels[i + k] =
                (self.pixels[i + k] as f32 * (1. - alpha) + c[k] as f32 * alpha) as u8;
        }
        self.pixels[i + 3] = 255;
    }
}
impl GraphPainter for RasterGraphPainter<'_> {
    fn round_rect(&mut self, rect: GraphRect, radius: f32, color: GraphColor) {
        let d = self.density;
        let [left, top] = [rect.origin[0] * d, rect.origin[1] * d];
        let [right, bottom] = [left + rect.size[0] * d, top + rect.size[1] * d];
        if ![left, top, right, bottom, radius]
            .iter()
            .all(|n| n.is_finite())
            || right <= left
            || bottom <= top
        {
            return;
        }
        let radius = (radius * d)
            .max(0.)
            .min((right - left) * 0.5)
            .min((bottom - top) * 0.5);
        // Opaque interiors are row copies; only boundary coverage needs blending.
        // This avoids allocating and compositing a full-area vector mask per fill.
        let solid = color.repeat(self.width);
        for y in (top.floor() as i32).max(0)..(bottom.ceil() as i32).min(self.height as i32) {
            let yc = (y as f32 + 0.5).clamp(top, bottom);
            let dy = if yc < top + radius {
                top + radius - yc
            } else if yc > bottom - radius {
                yc - (bottom - radius)
            } else {
                0.
            };
            let inset = radius - (radius * radius - dy * dy).max(0.).sqrt();
            let l = left + inset;
            let r = right - inset;
            let vertical = ((y as f32 + 1.).min(bottom) - (y as f32).max(top)).clamp(0., 1.);
            let start = (l.ceil() as i32).max(0).min(self.width as i32);
            let end = (r.floor() as i32).max(start).min(self.width as i32);
            if color[3] == 255 && vertical == 1. {
                let offset = (y as usize * self.width + start as usize) * 4;
                self.pixels[offset..offset + (end - start) as usize * 4]
                    .copy_from_slice(&solid[..(end - start) as usize * 4]);
            } else {
                for x in start..end {
                    self.blend(x, y, color, vertical * color[3] as f32 / 255.);
                }
            }
            for x in [l.floor() as i32, r.floor() as i32] {
                if x >= start && x < end {
                    continue;
                }
                let coverage =
                    ((x as f32 + 1.).min(r) - (x as f32).max(l)).clamp(0., 1.) * vertical;
                if coverage > 0. {
                    self.blend(x, y, color, coverage * color[3] as f32 / 255.);
                }
                if l.floor() == r.floor() {
                    break;
                }
            }
        }
    }
    fn text_width(&mut self, text: &str, size: f32) -> f32 {
        let size = (size * self.density * 4.).round().max(4.) / 4.;
        text.chars()
            .map(|ch| self.resources.font.metrics(ch, size).advance_width)
            .sum::<f32>()
            / self.density
    }

    fn curve(&mut self, p: [Point; 4], width: f32, color: GraphColor) {
        let d = self.density;
        let mut path = ThePath::new();
        path.move_to((p[0][0] * d, p[0][1] * d));
        path.curve_to(
            (p[1][0] * d, p[1][1] * d),
            (p[2][0] * d, p[2][1] * d),
            (p[3][0] * d, p[3][1] * d),
        );
        let mut surface = TheSurfaceMut::new(&mut *self.pixels, self.width, self.height).unwrap();
        self.resources.painter.stroke_path(
            &mut surface,
            &path,
            &ThePathStroke::new(width * d, ThePaint::solid(color)),
        );
    }
    fn text(&mut self, rect: GraphRect, text: &str, size: f32, color: GraphColor) {
        let d = self.density;
        let size = (size * d * 4.).round().max(4.) / 4.;
        // Bound cache while continuously exploring zoom levels.
        if self.resources.glyphs.len() > 4096 {
            self.resources.glyphs.clear();
        }
        let mut x = rect.origin[0] * d;
        let baseline = rect.origin[1] * d + size;
        for ch in text.chars() {
            let key = (ch, size.to_bits());
            let (m, bitmap) = self
                .resources
                .glyphs
                .entry(key)
                .or_insert_with(|| self.resources.font.rasterize(ch, size));
            // Borrow framebuffer independently so cached glyph storage is not copied.
            for yy in 0..m.height {
                for xx in 0..m.width {
                    let px = x as i32 + m.xmin + xx as i32;
                    let py = baseline as i32 - m.height as i32 - m.ymin + yy as i32;
                    if px < 0
                        || py < 0
                        || px >= self.width as i32
                        || py >= self.height as i32
                        || (px as f32) < rect.origin[0] * d
                        || px as f32 >= (rect.origin[0] + rect.size[0]) * d
                        || (py as f32) < rect.origin[1] * d
                        || py as f32 >= (rect.origin[1] + rect.size[1]) * d
                    {
                        continue;
                    }
                    let alpha = bitmap[yy * m.width + xx] as f32 / 255. * color[3] as f32 / 255.;
                    let i = (py as usize * self.width + px as usize) * 4;
                    for k in 0..3 {
                        self.pixels[i + k] = (self.pixels[i + k] as f32 * (1. - alpha)
                            + color[k] as f32 * alpha)
                            as u8;
                    }
                    self.pixels[i + 3] = 255;
                }
            }
            x += m.advance_width;
            if x > (rect.origin[0] + rect.size[0]) * d {
                break;
            }
        }
    }
    fn preview(&mut self, rect: GraphRect, key: &str) {
        let Some(image) = self.assets.image(key) else {
            return;
        };
        let w = image.pixel_width();
        let h = image.pixel_height();
        if w == 0 || h == 0 {
            return;
        }
        let d = self.density;
        let x0 = (rect.origin[0] * d) as i32;
        let y0 = (rect.origin[1] * d) as i32;
        let dw = (rect.size[0] * d).max(1.) as i32;
        let dh = (rect.size[1] * d).max(1.) as i32;
        for y in y0.max(0)..(y0 + dh).min(self.height as i32) {
            for x in x0.max(0)..(x0 + dw).min(self.width as i32) {
                let sx = ((x - x0) as usize * w / dw as usize).min(w - 1);
                let sy = ((y - y0) as usize * h / dh as usize).min(h - 1);
                let c: [u8; 4] = image.pixels()[(sy * w + sx) * 4..(sy * w + sx) * 4 + 4]
                    .try_into()
                    .unwrap();
                self.blend(x, y, c, c[3] as f32 / 255.);
            }
        }
    }
}
