use crate::prelude::*;

pub struct TheText {
    id: TheId,

    limiter: TheSizeLimiter,

    dim: TheDim,
    text: String,
    text_size: f32,
    text_color: RGBA,

    fixed_size_text: String,

    is_dirty: bool,
}

impl TheWidget for TheText {
    fn new(id: TheId) -> Self
    where
        Self: Sized,
    {
        let mut limiter = TheSizeLimiter::new();
        limiter.set_max_height(21);

        Self {
            id,
            limiter,

            dim: TheDim::zero(),
            text: "".to_string(),
            text_size: 13.0,
            text_color: WHITE,

            fixed_size_text: String::default(),

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

    fn set_value(&mut self, value: TheValue) {
        if let TheValue::Text(value) = value {
            self.set_text(value);
        }
    }

    fn calculate_size(&mut self, ctx: &mut TheContext) {
        if self.fixed_size_text.is_empty() {
            if !self.text.is_empty() {
                let size = ctx.draw.get_text_size(
                    &self.text,
                    &TheFontSettings {
                        size: self.text_size,
                        ..Default::default()
                    },
                );
                self.limiter_mut()
                    .set_max_size(Vec2::new(size.0 as i32 + 1, size.1 as i32 + 1));
            } else {
                self.limiter_mut().set_max_size(Vec2::new(20, 20));
            }
        } else {
            let size = ctx.draw.get_text_size(
                &self.fixed_size_text,
                &TheFontSettings {
                    size: self.text_size,
                    ..Default::default()
                },
            );
            self.limiter_mut()
                .set_max_size(Vec2::new(size.0 as i32 + 1, size.1 as i32 + 1));
        }
    }

    fn draw(
        &mut self,
        buffer: &mut TheRGBABuffer,
        _style: &mut Box<dyn TheStyle>,
        ctx: &mut TheContext,
    ) {
        if !self.dim().is_valid() {
            return;
        }

        let stride = buffer.stride();

        let mut shrinker = TheDimShrinker::zero();
        shrinker.shrink_by(0, 1, 0, 0);

        ctx.draw.text_rect_blend(
            buffer.pixels_mut(),
            &self.dim.to_buffer_shrunk_utuple(&shrinker),
            stride,
            &self.text,
            TheFontSettings {
                size: self.text_size,
                ..Default::default()
            },
            &self.text_color,
            TheHorizontalAlign::Left,
            TheVerticalAlign::Center,
        );

        self.is_dirty = false;
    }

    fn as_text(&mut self) -> Option<&mut dyn TheTextTrait> {
        Some(self)
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// TheText specific functions.
pub trait TheTextTrait {
    /// Set the text to display.
    fn set_text(&mut self, text: String);
    /// Set the text size.
    fn set_text_size(&mut self, text_size: f32);
    /// Sets the text color.
    fn set_text_color(&mut self, color: RGBA);
    /// Set fixed size text.
    fn set_fixed_size_text(&mut self, fixed_size_text: String);
}

impl TheTextTrait for TheText {
    fn set_text(&mut self, text: String) {
        self.text = text;
        self.is_dirty = true;
    }
    fn set_text_size(&mut self, text_size: f32) {
        self.text_size = text_size;
        self.is_dirty = true;
    }
    fn set_text_color(&mut self, color: RGBA) {
        self.text_color = color;
        self.is_dirty = true;
    }
    fn set_fixed_size_text(&mut self, fixed_size_text: String) {
        self.fixed_size_text = fixed_size_text;
        self.is_dirty = true;
    }
}
