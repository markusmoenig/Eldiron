use crate::prelude::*;

pub struct TheSoft3DView {
    id: TheId,
    limiter: TheSizeLimiter,
    state: TheWidgetState,

    dim: TheDim,
    color: RGBA,
    is_dirty: bool,
}

impl TheWidget for TheSoft3DView {
    fn new(id: TheId) -> Self
    where
        Self: Sized,
    {
        let mut limiter = TheSizeLimiter::new();
        limiter.set_max_size(vec2i(20, 20));
        Self {
            id,
            limiter,
            state: TheWidgetState::None,

            dim: TheDim::zero(),
            color: WHITE,
            is_dirty: false,
        }
    }

    fn id(&self) -> &TheId {
        &self.id
    }

    #[allow(clippy::single_match)]
    fn on_event(&mut self, event: &TheEvent, ctx: &mut TheContext) -> bool {
        let mut redraw = false;
        // println!("event ({}): {:?}", self.widget_id.name, event);
        match event {
            TheEvent::MouseDown(_coord) => {
                if self.state == TheWidgetState::Selected {
                    self.state = TheWidgetState::None;
                    ctx.ui.send_widget_state_changed(self.id(), self.state);
                } else if self.state != TheWidgetState::Selected {
                    self.state = TheWidgetState::Selected;
                    ctx.ui.send_widget_state_changed(self.id(), self.state);
                }
                self.is_dirty = true;
                redraw = true;
            }
            _ => {}
        }
        redraw
    }

    fn dim(&self) -> &TheDim {
        &self.dim
    }

    fn dim_mut(&mut self) -> &mut TheDim {
        &mut self.dim
    }

    fn set_dim(&mut self, dim: TheDim) {
        if self.dim != dim {
            self.dim = dim;
            self.is_dirty = true;
        }
    }

    fn set_state(&mut self, state: TheWidgetState) {
        self.state = state;
        self.is_dirty = true;
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
        _style: &mut Box<dyn TheStyle>,
        ctx: &mut TheContext,
    ) {
        let stride: usize = buffer.stride();
        let mut shrinker = TheDimShrinker::zero();

        if !self.dim().is_valid() {
            return;
        }

        //style.draw_widget_border(buffer, self, &mut shrinker, ctx);

        ctx.draw.rect_outline_border(
            buffer.pixels_mut(),
            &self.dim.to_buffer_shrunk_utuple(&shrinker),
            stride,
            &self.color,
            1,
        );

        if self.state == TheWidgetState::Selected {
            shrinker.shrink(1);
            ctx.draw.rect(
                buffer.pixels_mut(),
                &self.dim.to_buffer_shrunk_utuple(&shrinker),
                stride,
                &self.color,
            );
            ctx.draw.rect(
                buffer.pixels_mut(),
                &self.dim.to_buffer_shrunk_utuple(&shrinker),
                stride,
                &self.color,
            );
        }

        self.is_dirty = false;
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub trait TheSoft3DViewTrait {
    fn set_color(&mut self, color: RGBA);
}

impl TheSoft3DViewTrait for TheSoft3DView {
    fn set_color(&mut self, color: RGBA) {
        self.color = color;
        println!("ddad");
    }
}