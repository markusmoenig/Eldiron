use crate::prelude::*;

pub struct TheStatusbar {
    #[cfg(feature = "text")]
    paint_revision: Uuid,
    id: TheId,

    limiter: TheSizeLimiter,
    text: String,

    dim: TheDim,
    is_dirty: bool,
}

impl TheStatusbar {
    fn invalidate_retained_paint(&mut self) {
        self.is_dirty = true;
        #[cfg(feature = "text")]
        {
            self.paint_revision = Uuid::new_v4();
        }
    }
}

impl TheWidget for TheStatusbar {
    #[cfg(feature = "text")]
    fn retained_paint_revision(&self) -> Option<Uuid> {
        (self.text.len() <= 256).then_some(self.paint_revision)
    }

    #[cfg(feature = "text")]
    fn paint(
        &mut self,
        p: &mut crate::theui::paint::WidgetPainter<'_>,
    ) -> crate::theui::paint::UiPaintResult {
        use crate::paint::Rect;
        use crate::theui::paint::bounds;
        let r = bounds(&self.dim);
        p.theme_fill(r, 0., StatusbarBackground);
        p.label(
            Rect::new(
                r.x + 20.,
                r.y + 1.,
                (r.width - 20.).max(0.),
                (r.height - 1.).max(0.),
            ),
            &self.text,
            13.5,
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
            #[cfg(feature = "text")]
            paint_revision: Uuid::new_v4(),
            id,
            limiter,
            text: "".to_string(),

            dim: TheDim::zero(),
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
            self.invalidate_retained_paint();
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
        let paint = style.theme().paint(StatusbarBackground, rect);
        let width = buffer.dim().width.max(0) as usize;
        let height = buffer.dim().height.max(0) as usize;
        if let Ok(mut surface) = TheSurfaceMut::new(buffer.draw_target(), width, height) {
            surface.set_clip(rect);
            ctx.painter.fill_rect(&mut surface, rect, &paint);
        }

        let stride = buffer.stride();
        let mut shrinker = TheDimShrinker::zero();
        shrinker.shrink_by(20, 1, 0, 0);

        ctx.draw.text_rect_blend(
            buffer.draw_target(),
            &self.dim.to_buffer_shrunk_utuple(&shrinker),
            stride,
            &self.text,
            TheFontSettings {
                size: 13.5,
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

    fn as_statusbar(&mut self) -> Option<&mut dyn TheStatusbarTrait> {
        Some(self)
    }
}

pub trait TheStatusbarTrait {
    fn set_text(&mut self, text: String);
}

impl TheStatusbarTrait for TheStatusbar {
    fn set_text(&mut self, text: String) {
        // The status bar is a single 21px row. Never pass multi-line
        // diagnostics to the text renderer: subsequent lines otherwise draw
        // over and outside the widget.
        let text = text
            .lines()
            .find(|line| !line.trim().is_empty())
            .map(str::trim)
            .unwrap_or_default()
            .to_string();
        if self.text != text {
            self.text = text;
            self.invalidate_retained_paint();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multiline_messages_are_reduced_to_one_status_line() {
        let mut statusbar = TheStatusbar::new(TheId::named("Statusbar"));
        statusbar.set_text("first line\nsecond line\nthird line".to_string());
        assert_eq!(statusbar.text, "first line");
    }
}
