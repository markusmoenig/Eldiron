use rusterix::{Pixel, Rect, Texture};
use theframework::{
    paint::{self, *},
    text::{TextLayout, TextSystem},
};

/// Authored source assets only; presenters never register a rasterized screen/HUD.
pub trait ScreenImages {
    fn rgba(
        &mut self,
        key: &str,
        width: u32,
        height: u32,
        pixels: &[u8],
    ) -> Result<ImageHandle, String>;
}

pub fn color(pixel: Pixel) -> Color {
    pixel.map(|c| c as f32 / 255.)
}
pub fn rect(r: Rect) -> paint::Rect {
    paint::Rect::new(r.x, r.y, r.width, r.height)
}
#[derive(Clone, Copy)]
pub enum Align {
    Left,
    Center,
    Right,
}

pub struct ScreenPainter<'a> {
    pub list: &'a mut PaintList,
    pub text: &'a mut TextSystem,
    pub images: &'a mut dyn ScreenImages,
}
impl ScreenPainter<'_> {
    pub fn fill(&mut self, bounds: Rect, pixel: Pixel) {
        if bounds.width > 0. && bounds.height > 0. && pixel[3] > 0 {
            self.list.rect(rect(bounds), 0., color(pixel));
        }
    }
    pub fn border(&mut self, r: Rect, width: f32, pixel: Pixel) {
        let w = width.min(r.width * 0.5).min(r.height * 0.5);
        if w <= 0. {
            return;
        }
        self.fill(Rect::new(r.x, r.y, r.width, w), pixel);
        self.fill(Rect::new(r.x, r.y + r.height - w, r.width, w), pixel);
        self.fill(Rect::new(r.x, r.y + w, w, r.height - w * 2.), pixel);
        self.fill(
            Rect::new(r.x + r.width - w, r.y + w, w, r.height - w * 2.),
            pixel,
        );
    }
    pub fn label(
        &mut self,
        bounds: Rect,
        content: &str,
        family: &str,
        size: f32,
        pixel: Pixel,
        align: Align,
        centered: bool,
    ) -> TextLayout {
        let family = if family.trim().is_empty() {
            "Roboto"
        } else {
            family
        };
        let layout = self.text.layout(content, family, size, None);
        let x = bounds.x
            + match align {
                Align::Left => 0.,
                Align::Center => (bounds.width - layout.width()) * 0.5,
                Align::Right => bounds.width - layout.width(),
            };
        let y = bounds.y
            + if centered {
                (bounds.height - layout.height()) * 0.5
            } else {
                0.
            };
        self.list.ops.push(PaintOp::PushClip(rect(bounds)));
        for run in &layout.runs {
            self.list.ops.push(PaintOp::GlyphRun {
                run: run.clone(),
                origin: [x, y],
                color: color(pixel),
            });
        }
        self.list.ops.push(PaintOp::PopClip);
        layout
    }
    pub fn texture(
        &mut self,
        key: &str,
        texture: &Texture,
        bounds: Rect,
        uv: paint::Rect,
        sampling: Sampling,
        tint: Color,
    ) -> Result<(), String> {
        if texture.width == 0 || texture.height == 0 || bounds.width <= 0. || bounds.height <= 0. {
            return Ok(());
        }
        let image = self.images.rgba(
            key,
            texture.width as u32,
            texture.height as u32,
            &texture.data,
        )?;
        self.list.ops.push(PaintOp::Image {
            image,
            rect: rect(bounds),
            uv,
            sampling,
            tint,
        });
        Ok(())
    }
}
