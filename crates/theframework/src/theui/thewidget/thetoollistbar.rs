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

        let stride = buffer.stride();
        let utuple: (usize, usize, usize, usize) = self.dim.to_buffer_utuple();

        ctx.draw.rect(
            buffer.pixels_mut(),
            &(utuple.0, utuple.1, utuple.2 - 1, 1),
            stride,
            style.theme().color(DefaultWidgetDarkBackground),
        );

        ctx.draw.rect(
            buffer.pixels_mut(),
            &(utuple.0 + utuple.2 - 1, utuple.1, 1, utuple.3),
            stride,
            style.theme().color(DefaultWidgetDarkBackground),
        );

        if let Some(icon) = ctx.ui.icon("dark_toollistbar") {
            for x in 0..utuple.2 - 1 {
                let r = (utuple.0 + x, utuple.1 + 1, 1, icon.dim().height as usize);
                ctx.draw
                    .copy_slice(buffer.pixels_mut(), icon.pixels(), &r, stride);
            }
        }

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
