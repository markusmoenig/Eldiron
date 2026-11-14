//! Compatibility bitmap data and explicit legacy font rasterization.
//! Native paint/text paths use ImageAsset and Swash/Parley instead.
use crate::prelude::*;
use crate::thesurface::RasterScale;
use fontdue::layout::{
    CoordinateSystem, HorizontalAlign, Layout, LayoutSettings, TextStyle, VerticalAlign,
};
pub use theframework_core::thergbabuffer::*;

/// Reference/adapter-only text painting. Bitmap data remains CPU-core owned.
pub trait LegacyBitmapText {
    fn draw_text(
        &mut self,
        position: Vec2<i32>,
        font: &fontdue::Font,
        text: &str,
        size: f32,
        color: [u8; 4],
        halign: TheHorizontalAlign,
        valign: TheVerticalAlign,
    );
}
impl LegacyBitmapText for TheRGBABuffer {
    #[allow(clippy::too_many_arguments)]
    /// Render an aligned text in the buffer.
    fn draw_text(
        &mut self,
        position: Vec2<i32>,
        font: &fontdue::Font,
        text: &str,
        size: f32,
        color: [u8; 4],
        halign: TheHorizontalAlign,
        valign: TheVerticalAlign,
    ) {
        if self.render_scale() != 1.0 {
            let s = RasterScale(self.render_scale());
            self.with_physical(|b| {
                b.draw_text(
                    Vec2::new(s.i(position.x), s.i(position.y)),
                    font,
                    text,
                    size * s.0,
                    color,
                    halign,
                    valign,
                )
            });
            return;
        }

        pub fn mix_color(a: &[u8; 4], b: &[u8; 4], v: f32) -> [u8; 4] {
            [
                (((1.0 - v) * (a[0] as f32 / 255.0) + b[0] as f32 / 255.0 * v) * 255.0) as u8,
                (((1.0 - v) * (a[1] as f32 / 255.0) + b[1] as f32 / 255.0 * v) * 255.0) as u8,
                (((1.0 - v) * (a[2] as f32 / 255.0) + b[2] as f32 / 255.0 * v) * 255.0) as u8,
                (((1.0 - v) * (a[3] as f32 / 255.0) + b[3] as f32 / 255.0 * v) * 255.0) as u8,
            ]
        }

        // fn get_text_size(font: &Font, size: f32, text: &str) -> (usize, usize) {
        //     let fonts = &[font];

        //     let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
        //     layout.reset(&LayoutSettings {
        //         ..LayoutSettings::default()
        //     });
        //     layout.append(fonts, &TextStyle::new(text, size, 0));

        //     let x = layout.glyphs()[layout.glyphs().len() - 1].x.ceil() as usize
        //         + layout.glyphs()[layout.glyphs().len() - 1].width
        //         + 1;
        //     (x, layout.height() as usize)
        // }

        // let (width, height) = get_text_size(font, size, text);

        let fonts = &[font];

        let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
        layout.reset(&LayoutSettings {
            max_width: Some(self.dim().width as f32),
            max_height: Some(self.dim().height as f32),
            horizontal_align: if halign == TheHorizontalAlign::Left {
                HorizontalAlign::Left
            } else if halign == TheHorizontalAlign::Right {
                HorizontalAlign::Right
            } else {
                HorizontalAlign::Center
            },
            vertical_align: if valign == TheVerticalAlign::Top {
                VerticalAlign::Top
            } else if valign == TheVerticalAlign::Bottom {
                VerticalAlign::Bottom
            } else {
                VerticalAlign::Middle
            },
            ..LayoutSettings::default()
        });
        layout.append(fonts, &TextStyle::new(text, size, 0));

        for glyph in layout.glyphs() {
            let (metrics, alphamap) = font.rasterize(glyph.parent, glyph.key.px);
            for y in 0..metrics.height {
                for x in 0..metrics.width {
                    let m = alphamap[x + y * metrics.width];

                    if let Some(index) = self.pixel_index(
                        x as i32 + glyph.x as i32 + position.x,
                        y as i32 + glyph.y as i32 + position.y,
                    ) {
                        let background = &[
                            self.pixels()[index],
                            self.pixels()[index + 1],
                            self.pixels()[index + 2],
                            self.pixels()[index + 3],
                        ];
                        self.pixels_mut()[index..index + 4].copy_from_slice(&mix_color(
                            background,
                            &color,
                            m as f32 / 255.0,
                        ));
                    }
                }
            }
        }
    }
}
