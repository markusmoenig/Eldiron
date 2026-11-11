use crate::prelude::*;

fn draw_switchbar_marker<'a>(
    pixels: impl Into<TheRasterTarget<'a>>,
    width: usize,
    height: usize,
    bounds: ThePixelRect,
    paint: &ThePaint,
    painter: &mut ThePainter,
) {
    let Ok(mut surface) = TheSurfaceMut::new(pixels, width, height) else {
        return;
    };
    surface.set_clip(bounds);

    let center_y = bounds.y as f32 + bounds.height as f32 * 0.5;
    let mut path = ThePath::new();
    for offset in [0.0, 4.0] {
        let x = bounds.x as f32 + 7.0 + offset;
        path.move_to((x, center_y - 3.5))
            .line_to((x + 3.5, center_y))
            .line_to((x, center_y + 3.5));
    }
    painter.stroke_path(
        &mut surface,
        &path,
        &ThePathStroke::new(1.5, paint.clone())
            .with_cap(TheLineCap::Round)
            .with_join(TheLineJoin::Round),
    );
}

pub struct TheSwitchbar {
    id: TheId,

    limiter: TheSizeLimiter,

    dim: TheDim,
    text: String,
    is_dirty: bool,
}

impl TheWidget for TheSwitchbar {
    #[cfg(feature = "text")]
    fn paint(
        &mut self,
        p: &mut crate::theui::paint::WidgetPainter<'_>,
    ) -> crate::theui::paint::UiPaintResult {
        use crate::paint::{LineCap, LineJoin, PaintOp, Path, PathElement, Rect, Stroke};
        use std::sync::Arc;
        let r = crate::theui::paint::bounds(&self.dim);
        p.fill(r, SwitchbarBorder);
        let brush = p.theme_brush(r, SwitchbarChrome);
        p.list.rect(r.inset(1.), 0., brush);
        let y = r.y + r.height * 0.5;
        let mut elements = Vec::new();
        for offset in [0., 4.] {
            let x = r.x + 7. + offset;
            elements.extend([
                PathElement::Move([x, y - 3.5]),
                PathElement::Line([x + 3.5, y]),
                PathElement::Line([x, y + 3.5]),
            ]);
        }
        p.list.ops.push(PaintOp::Stroke {
            path: Arc::new(Path { elements }),
            stroke: Stroke {
                width: 1.5,
                cap: LineCap::Round,
                join: LineJoin::Round,
                brush: p.theme_brush(r, SwitchbarMarker),
            },
        });
        p.label(
            Rect::new(
                r.x + 30.,
                r.y + 1.,
                (r.width - 30.).max(0.),
                (r.height - 1.).max(0.),
            ),
            &self.text,
            13.,
            *p.theme.color(SectionbarNormalTextColor),
            TheHorizontalAlign::Left,
            TheVerticalAlign::Center,
        );
        self.is_dirty = false;
        Ok(())
    }

    fn new(id: TheId) -> Self
    where
        Self: Sized,
    {
        let mut limiter = TheSizeLimiter::new();
        limiter.set_max_height(21);

        Self {
            id,
            limiter,

            dim: TheDim::zero(),
            text: "".to_string(),
            is_dirty: false,
        }
    }

    fn id(&self) -> &TheId {
        &self.id
    }

    // fn on_event(&mut self, event: &TheEvent, ctx: &mut TheContext) -> bool {
    //     false
    // }

    fn dim(&self) -> &TheDim {
        &self.dim
    }

    fn dim_mut(&mut self) -> &mut TheDim {
        &mut self.dim
    }

    fn set_dim(&mut self, dim: TheDim, _ctx: &mut TheContext) {
        if self.dim != dim {
            self.dim = dim;
            self.is_dirty = true;
        }
    }

    fn limiter(&self) -> &TheSizeLimiter {
        &self.limiter
    }

    fn limiter_mut(&mut self) -> &mut TheSizeLimiter {
        &mut self.limiter
    }

    fn needs_redraw(&mut self) -> bool {
        self.is_dirty
    }

    fn set_value(&mut self, value: TheValue) {
        if let Some(text) = value.to_string() {
            self.text = text;
            self.is_dirty = true;
            self.is_dirty = true;
        }
    }

    fn draw(
        &mut self,
        buffer: &mut TheRGBABuffer,
        style: &mut Box<dyn TheStyle>,
        ctx: &mut TheContext,
    ) {
        if !self.dim().is_valid() {
            return;
        }

        let rect = ThePixelRect::new(
            self.dim.buffer_x,
            self.dim.buffer_y,
            self.dim.width,
            self.dim.height,
        );
        let border = *style.theme().color(SwitchbarBorder);
        let paint = style.theme().paint(SwitchbarChrome, rect);
        let marker = style.theme().paint(SwitchbarMarker, rect);
        let inner = ThePixelRect::new(
            rect.x.saturating_add(1),
            rect.y.saturating_add(1),
            rect.width.saturating_sub(2),
            rect.height.saturating_sub(2),
        );
        let width = buffer.dim().width.max(0) as usize;
        let height = buffer.dim().height.max(0) as usize;
        if let Ok(mut surface) = TheSurfaceMut::new(buffer.draw_target(), width, height) {
            surface.set_clip(rect);
            surface.fill_rect(rect, border);
            ctx.painter.fill_rect(&mut surface, inner, &paint);
        }

        let stride = buffer.stride();
        draw_switchbar_marker(
            buffer.draw_target(),
            width,
            height,
            rect,
            &marker,
            &mut ctx.painter,
        );

        let mut shrinker = TheDimShrinker::zero();
        shrinker.shrink_by(30, 1, 0, 0);

        let mut r = self.dim.to_buffer_shrunk_utuple(&shrinker);
        r.3 = 21;
        ctx.draw.text_rect_blend(
            buffer.draw_target(),
            &r,
            stride,
            &self.text,
            TheFontSettings {
                size: 13.0,
                ..Default::default()
            },
            style.theme().color(SectionbarNormalTextColor),
            TheHorizontalAlign::Left,
            TheVerticalAlign::Center,
        );

        self.is_dirty = false;
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub trait TheSwitchbarTrait {
    fn set_text(&mut self, text: String);
}

impl TheSwitchbarTrait for TheSwitchbar {
    fn set_text(&mut self, text: String) {
        self.text = text;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn procedural_marker_clips_and_preserves_guard_bytes() {
        const WIDTH: usize = 7;
        const HEIGHT: usize = 6;
        const GUARD: usize = 29;
        const SENTINEL: u8 = 0xa9;

        let body_len = WIDTH * HEIGHT * 4;
        let mut pixels = vec![0; body_len + GUARD];
        pixels[body_len..].fill(SENTINEL);
        draw_switchbar_marker(
            &mut pixels,
            WIDTH,
            HEIGHT,
            ThePixelRect::new(-8, -6, 30, 21),
            &ThePaint::solid([238, 240, 242, 255]),
            &mut ThePainter::new(),
        );

        assert!(pixels[..body_len].iter().any(|byte| *byte != 0));
        assert!(pixels[body_len..].iter().all(|byte| *byte == SENTINEL));
    }
}
