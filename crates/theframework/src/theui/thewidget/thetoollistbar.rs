use crate::prelude::*;

pub struct TheToolListBar {
    id: TheId,
    limiter: TheSizeLimiter,

    text: String,

    dim: TheDim,
    is_dirty: bool,
}

impl TheWidget for TheToolListBar {
    fn new(id: TheId) -> Self
    where
        Self: Sized,
    {
        let mut limiter = TheSizeLimiter::new();
        limiter.set_max_height(23);

        Self {
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
        if let Ok(mut surface) = TheSurfaceMut::new(buffer.pixels_mut(), width, height) {
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
            buffer.pixels_mut(),
            &utuple,
            stride,
            &self.text,
            TheFontSettings {
                size: 11.5,
                ..Default::default()
            },
            &WHITE,
            TheHorizontalAlign::Center,
            TheVerticalAlign::Center,
        );

        self.is_dirty = false;
    }

    fn set_value(&mut self, value: TheValue) {
        if let TheValue::Text(value) = value {
            self.text = value;
        }
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
