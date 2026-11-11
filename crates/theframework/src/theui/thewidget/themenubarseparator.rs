use crate::prelude::*;

pub struct TheMenubarSeparator {
    id: TheId,
    limiter: TheSizeLimiter,

    dim: TheDim,
    is_dirty: bool,
}

impl TheWidget for TheMenubarSeparator {
    #[cfg(feature = "render-wgpu")]
    fn paint(
        &mut self,
        p: &mut crate::theui::paint::WidgetPainter<'_>,
    ) -> crate::theui::paint::UiPaintResult {
        use crate::paint::Rect;
        use crate::theui::paint::bounds;
        let r = bounds(&self.dim);
        p.fill(
            Rect::new(r.x + 4., r.y, 1., r.height),
            MenubarButtonSeparator1,
        );
        p.fill(
            Rect::new(r.x + 5., r.y, 1., r.height),
            MenubarButtonSeparator2,
        );
        self.is_dirty = false;
        Ok(())
    }
    fn new(id: TheId) -> Self
    where
        Self: Sized,
    {
        let mut limiter = TheSizeLimiter::new();
        limiter.set_max_size(Vec2::new(10, 33));

        Self {
            id,
            limiter,

            dim: TheDim::zero(),
            is_dirty: false,
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

    fn set_needs_redraw(&mut self, redraw: bool) {
        self.is_dirty = redraw;
    }

    fn draw(
        &mut self,
        buffer: &mut TheRGBABuffer,
        style: &mut Box<dyn TheStyle>,
        _ctx: &mut TheContext,
    ) {
        if !self.dim().is_valid() {
            return;
        }

        let first = ThePixelRect::new(
            self.dim.buffer_x.saturating_add(4),
            self.dim.buffer_y,
            1,
            self.dim.height,
        );
        let second = ThePixelRect::new(first.x.saturating_add(1), first.y, 1, first.height);
        let width = buffer.dim().width.max(0) as usize;
        let height = buffer.dim().height.max(0) as usize;
        if let Ok(mut surface) = TheSurfaceMut::new(buffer.draw_target(), width, height) {
            surface.set_clip(ThePixelRect::new(
                self.dim.buffer_x,
                self.dim.buffer_y,
                self.dim.width,
                self.dim.height,
            ));
            surface.fill_rect(first, *style.theme().color(MenubarButtonSeparator1));
            surface.fill_rect(second, *style.theme().color(MenubarButtonSeparator2));
        }

        self.is_dirty = false;
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
