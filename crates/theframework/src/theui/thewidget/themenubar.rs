use crate::prelude::*;

pub struct TheMenubar {
    id: TheId,

    limiter: TheSizeLimiter,

    dim: TheDim,
    text: String,
    is_dirty: bool,
}

impl TheWidget for TheMenubar {
    fn new(id: TheId) -> Self
    where
        Self: Sized,
    {
        let mut limiter = TheSizeLimiter::new();
        limiter.set_max_height(43);

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

    fn draw(
        &mut self,
        buffer: &mut TheRGBABuffer,
        style: &mut Box<dyn TheStyle>,
        ctx: &mut TheContext,
    ) {
        if !self.dim().is_valid() {
            return;
        }

        let bounds = ThePixelRect::new(
            self.dim.buffer_x,
            self.dim.buffer_y,
            self.dim.width,
            self.dim.height,
        );
        let fill = style.theme().paint(MenubarBackground, bounds);
        let width = buffer.dim().width.max(0) as usize;
        let height = buffer.dim().height.max(0) as usize;
        if let Ok(mut surface) = TheSurfaceMut::new(buffer.pixels_mut(), width, height) {
            surface.set_clip(bounds);
            ctx.painter.fill_rect(&mut surface, bounds, &fill);
        }

        self.is_dirty = false;
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub trait TheMenubarTrait {
    fn set_text(&mut self, text: String);
}

impl TheMenubarTrait for TheMenubar {
    fn set_text(&mut self, text: String) {
        self.text = text;
    }
}
