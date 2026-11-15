use crate::prelude::*;

pub struct TheToolListBar {
    #[cfg(feature = "text")]
    paint_revision: Uuid,
    id: TheId,
    limiter: TheSizeLimiter,

    text: String,

    dim: TheDim,
    is_dirty: bool,
}

impl TheToolListBar {
    fn invalidate_retained_paint(&mut self) {
        self.is_dirty = true;
        #[cfg(feature = "text")]
        {
            self.paint_revision = Uuid::new_v4();
        }
    }
}

impl TheWidget for TheToolListBar {
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
        p.theme_fill(r, 0., ToolListBarChrome);
        p.fill(
            Rect::new(r.x, r.y, r.width, 1.),
            DefaultWidgetDarkBackground,
        );
        p.fill(
            Rect::new(r.x + r.width - 1., r.y, 1., r.height),
            DefaultWidgetDarkBackground,
        );
        p.label(
            r,
            &self.text,
            11.5,
            *p.theme.color(SectionbarNormalTextColor),
            TheHorizontalAlign::Center,
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
        limiter.set_max_height(23);

        Self {
            #[cfg(feature = "text")]
            paint_revision: Uuid::new_v4(),
            id,
            limiter,

            text: "TOOLS".into(),

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
        let paint = style.theme().paint(ToolListBarChrome, rect);
        let border = *style.theme().color(DefaultWidgetDarkBackground);
        let width = buffer.dim().width.max(0) as usize;
        let height = buffer.dim().height.max(0) as usize;
        if let Ok(mut surface) = TheSurfaceMut::new(buffer.draw_target(), width, height) {
            surface.set_clip(rect);
            ctx.painter.fill_rect(&mut surface, rect, &paint);
            surface.fill_rect(ThePixelRect::new(rect.x, rect.y, rect.width, 1), border);
            surface.fill_rect(
                ThePixelRect::new(
                    rect.x.saturating_add(rect.width.saturating_sub(1)),
                    rect.y,
                    1,
                    rect.height,
                ),
                border,
            );
        }

        let stride = buffer.stride();
        let utuple: (usize, usize, usize, usize) = self.dim.to_buffer_utuple();
        ctx.draw.text_rect_blend(
            buffer.draw_target(),
            &utuple,
            stride,
            &self.text,
            TheFontSettings {
                size: 11.5,
                ..Default::default()
            },
            style.theme().color(SectionbarNormalTextColor),
            TheHorizontalAlign::Center,
            TheVerticalAlign::Center,
        );

        self.is_dirty = false;
    }

    fn set_value(&mut self, value: TheValue) {
        if let TheValue::Text(value) = value {
            if self.text != value {
                self.text = value;
                self.invalidate_retained_paint();
            }
        }
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
