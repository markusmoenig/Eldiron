use crate::prelude::*;

pub struct TheStatusbar {
    id: TheId,

    limiter: TheSizeLimiter,
    text: String,

    dim: TheDim,
    is_dirty: bool,
}

impl TheWidget for TheStatusbar {
    fn new(id: TheId) -> Self
    where
        Self: Sized,
    {
        let mut limiter = TheSizeLimiter::new();
        limiter.set_max_height(21);

        Self {
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
            &(utuple.0, utuple.1, 1, utuple.3),
            stride,
            style.theme().color(StatusbarStart),
        );

        if let Some(icon) = ctx.ui.icon("dark_statusbar") {
            for x in 1..utuple.2 {
                let r = (utuple.0 + x, utuple.1, 1, icon.dim().height as usize);
                ctx.draw
                    .copy_slice(buffer.pixels_mut(), icon.pixels(), &r, stride);
            }
        }

        let mut shrinker = TheDimShrinker::zero();
        shrinker.shrink_by(20, 1, 0, 0);

        ctx.draw.text_rect_blend(
            buffer.pixels_mut(),
            &self.dim.to_buffer_shrunk_utuple(&shrinker),
            stride,
            &self.text,
            TheFontSettings {
                size: 13.5,
                ..Default::default()
            },
            &WHITE,
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
        if self.text != text {
            self.text = text;
            self.is_dirty = true;
        }
    }
}
