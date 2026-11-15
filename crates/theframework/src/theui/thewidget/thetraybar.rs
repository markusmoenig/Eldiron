use crate::prelude::*;

pub struct TheTraybar {
    #[cfg(feature = "text")]
    paint_revision: Uuid,
    id: TheId,

    limiter: TheSizeLimiter,

    dim: TheDim,
    is_dirty: bool,
}

impl TheWidget for TheTraybar {
    #[cfg(feature = "text")]
    fn retained_paint_revision(&self) -> Option<Uuid> {
        Some(self.paint_revision)
    }

    #[cfg(feature = "text")]
    fn paint(
        &mut self,
        p: &mut crate::theui::paint::WidgetPainter<'_>,
    ) -> crate::theui::paint::UiPaintResult {
        use crate::paint::Rect;
        use crate::theui::paint::bounds;
        let r = bounds(&self.dim);
        p.fill(r, TraybarBorder);
        p.fill(
            Rect::new(
                r.x + 1.,
                r.y + 1.,
                (r.width - 1.).max(0.),
                (r.height - 1.).max(0.),
            ),
            TraybarBackground,
        );
        p.fill(
            Rect::new(r.x + 1., r.y + r.height - 1., (r.width - 1.).max(0.), 1.),
            TraybarBottomBorder,
        );
        self.is_dirty = false;
        Ok(())
    }

    fn new(id: TheId) -> Self
    where
        Self: Sized,
    {
        let mut limiter = TheSizeLimiter::new();
        limiter.set_max_height(27);

        Self {
            #[cfg(feature = "text")]
            paint_revision: Uuid::new_v4(),
            id,
            limiter,

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

    fn draw(
        &mut self,
        buffer: &mut TheRGBABuffer,
        style: &mut Box<dyn TheStyle>,
        ctx: &mut TheContext,
    ) {
        if !self.dim().is_valid() {
            return;
        }

        let stride = buffer.stride();
        let utuple = &self.dim.to_buffer_utuple();

        ctx.draw.rect(
            buffer.draw_target(),
            utuple,
            stride,
            style.theme().color(TraybarBorder),
        );

        ctx.draw.rect(
            buffer.draw_target(),
            &(utuple.0 + 1, utuple.1 + 1, utuple.2 - 1, utuple.3 - 1),
            stride,
            style.theme().color(TraybarBackground),
        );

        ctx.draw.rect(
            buffer.draw_target(),
            &(utuple.0 + 1, utuple.1 + utuple.3 - 1, utuple.2 - 1, 1),
            stride,
            style.theme().color(TraybarBottomBorder),
        );

        self.is_dirty = false;
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
