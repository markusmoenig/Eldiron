use crate::prelude::*;
use crate::thecontext::TheCursorIcon;

pub struct TheColorButton {
    id: TheId,
    limiter: TheSizeLimiter,
    state: TheWidgetState,

    status: Option<String>,

    dim: TheDim,
    color: RGBA,
    is_dirty: bool,
    cursor_icon: Option<TheCursorIcon>,
}

impl TheWidget for TheColorButton {
    fn new(id: TheId) -> Self
    where
        Self: Sized,
    {
        let mut limiter = TheSizeLimiter::new();
        limiter.set_max_size(Vec2::new(20, 20));
        Self {
            id,
            limiter,
            state: TheWidgetState::None,

            status: None,

            dim: TheDim::zero(),
            color: BLACK,
            is_dirty: false,
            cursor_icon: Some(TheCursorIcon::Hand),
        }
    }

    fn cursor_icon(&self) -> Option<TheCursorIcon> {
        self.cursor_icon
    }

    fn set_cursor_icon(&mut self, icon: Option<TheCursorIcon>) {
        self.cursor_icon = icon;
    }

    fn id(&self) -> &TheId {
        &self.id
    }

    fn status_text(&self) -> Option<String> {
        self.status.clone()
    }

    fn set_status_text(&mut self, text: &str) {
        self.status = Some(text.to_string());
    }

    #[allow(clippy::single_match)]
    fn on_event(&mut self, event: &TheEvent, ctx: &mut TheContext) -> bool {
        let mut redraw = false;
        // println!("event ({}): {:?}", self.widget_id.name, event);
        match event {
            TheEvent::MouseDown(_coord) => {
                self.state = TheWidgetState::Clicked;
                ctx.ui.send_widget_state_changed(self.id(), self.state);
                self.is_dirty = true;
                ctx.ui.send(TheEvent::ColorButtonClicked(self.id.clone()));
                redraw = true;
            }
            TheEvent::MouseUp(_coord) => {
                self.state = TheWidgetState::None;
                ctx.ui.send_widget_state_changed(self.id(), self.state);
                self.is_dirty = true;
                redraw = true;
            }
            TheEvent::Hover(_coord) => {
                if !self.id().equals(&ctx.ui.hover) {
                    self.is_dirty = true;
                    ctx.ui.set_hover(self.id());
                    redraw = true;
                }
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

    fn set_dim(&mut self, dim: TheDim, _ctx: &mut TheContext) {
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

    fn supports_hover(&mut self) -> bool {
        true
    }

    fn set_value(&mut self, value: TheValue) {
        if let TheValue::ColorObject(color) = value {
            self.color = color.to_u8_array();
            self.is_dirty = true;
        }
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

        if self.state == TheWidgetState::None {
            ctx.draw.rect_outline_border(
                buffer.pixels_mut(),
                &self.dim.to_buffer_shrunk_utuple(&shrinker),
                stride,
                &[80, 80, 80, 255],
                1,
            );
        } else {
            ctx.draw.rect_outline_border(
                buffer.pixels_mut(),
                &self.dim.to_buffer_shrunk_utuple(&shrinker),
                stride,
                &BLACK,
                1,
            );
        }

        shrinker.shrink(1);
        ctx.draw.rect(
            buffer.pixels_mut(),
            &self.dim.to_buffer_shrunk_utuple(&shrinker),
            stride,
            &self.color,
        );

        self.is_dirty = false;
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub trait TheColorColorButtonTrait {
    fn set_color(&mut self, color: RGBA);
}

impl TheColorColorButtonTrait for TheColorButton {
    fn set_color(&mut self, color: RGBA) {
        self.color = color;
    }
}
