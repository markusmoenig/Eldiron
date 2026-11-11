use crate::prelude::*;

pub struct TheHDivider {
    id: TheId,
    limiter: TheSizeLimiter,

    dim: TheDim,
}

impl TheWidget for TheHDivider {
    #[cfg(feature = "text")]
    fn paint(
        &mut self,
        p: &mut crate::theui::paint::WidgetPainter<'_>,
    ) -> crate::theui::paint::UiPaintResult {
        use crate::paint::Rect;
        use crate::theui::paint::bounds;
        let r = bounds(&self.dim);
        p.fill(
            Rect::new(r.x + (r.width / 2.).floor() - 1., r.y, 1., r.height),
            DividerStart,
        );
        p.fill(
            Rect::new(r.x + (r.width / 2.).floor(), r.y, 1., r.height),
            DividerEnd,
        );
        Ok(())
    }

    fn new(id: TheId) -> Self
    where
        Self: Sized,
    {
        let mut limiter = TheSizeLimiter::new();
        limiter.set_max_size(Vec2::new(16, 20));

        Self {
            id,
            limiter,

            dim: TheDim::zero(),
        }
    }

    fn id(&self) -> &TheId {
        &self.id
    }

    fn dim(&self) -> &TheDim {
        &self.dim
    }

    fn dim_mut(&mut self) -> &mut TheDim {
        &mut self.dim
    }

    fn set_dim(&mut self, dim: TheDim, _ctx: &mut TheContext) {
        if self.dim != dim {
            self.dim = dim;
        }
    }

    fn limiter(&self) -> &TheSizeLimiter {
        &self.limiter
    }

    fn limiter_mut(&mut self) -> &mut TheSizeLimiter {
        &mut self.limiter
    }

    fn draw(
        &mut self,
        buffer: &mut TheRGBABuffer,
        style: &mut Box<dyn TheStyle>,
        ctx: &mut TheContext,
    ) {
        let stride: usize = buffer.stride();

        if !self.dim().is_valid() {
            return;
        }

        let utuple = self.dim.to_buffer_utuple();

        let r: (usize, usize, usize, usize) = (utuple.0 + utuple.2 / 2 - 1, utuple.1, 1, utuple.3);

        ctx.draw.rect(
            buffer.draw_target(),
            &r,
            stride,
            style.theme().color(DividerStart),
        );

        let r: (usize, usize, usize, usize) = (utuple.0 + utuple.2 / 2, utuple.1, 1, utuple.3);

        ctx.draw.rect(
            buffer.draw_target(),
            &r,
            stride,
            style.theme().color(DividerEnd),
        );
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
