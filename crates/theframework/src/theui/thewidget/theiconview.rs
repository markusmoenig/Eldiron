use crate::prelude::*;

pub struct TheIconView {
    id: TheId,
    limiter: TheSizeLimiter,

    is_dirty: bool,
    tile: TheRGBATile,
    index: usize,

    text: Option<String>,
    text_size: f32,
    text_color: RGBA,

    border_color: Option<RGBA>,

    alpha_mode: bool,

    dim: TheDim,
}

impl TheWidget for TheIconView {
    fn new(id: TheId) -> Self
    where
        Self: Sized,
    {
        let mut limiter = TheSizeLimiter::new();
        limiter.set_max_size(Vec2::new(24, 24));

        Self {
            id,
            limiter,

            is_dirty: true,
            tile: TheRGBATile::default(),
            index: 0,

            text: None,
            text_size: 12.0,
            text_color: WHITE,

            border_color: None,

            alpha_mode: true,
            dim: TheDim::zero(),
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
                ctx.ui
                    .send_widget_state_changed(self.id(), TheWidgetState::Clicked);
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

    fn needs_redraw(&mut self) -> bool {
        self.is_dirty
    }

    fn draw(
        &mut self,
        buffer: &mut TheRGBABuffer,
        _style: &mut Box<dyn TheStyle>,
        ctx: &mut TheContext,
    ) {
        let stride: usize = buffer.stride();

        if !self.dim().is_valid() {
            return;
        }

        let utuple = self.dim.to_buffer_utuple();

        if !self.tile.buffer.is_empty() {
            if self.alpha_mode {
                ctx.draw.blend_scale_chunk(
                    buffer.pixels_mut(),
                    &(
                        utuple.0,
                        utuple.1,
                        self.dim.width as usize,
                        self.dim.height as usize,
                    ),
                    stride,
                    self.tile.buffer[self.index].pixels(),
                    &(
                        self.tile.buffer[0].dim().width as usize,
                        self.tile.buffer[0].dim().height as usize,
                    ),
                );
            } else {
                ctx.draw.scale_chunk(
                    buffer.pixels_mut(),
                    &(
                        utuple.0,
                        utuple.1,
                        self.dim.width as usize,
                        self.dim.height as usize,
                    ),
                    stride,
                    self.tile.buffer[self.index].pixels(),
                    &(
                        self.tile.buffer[0].dim().width as usize,
                        self.tile.buffer[0].dim().height as usize,
                    ),
                    1.0,
                );
            }
        }

        if let Some(text) = &self.text {
            ctx.draw.text_rect_blend(
                buffer.pixels_mut(),
                &utuple,
                stride,
                text,
                TheFontSettings {
                    size: self.text_size,
                    ..Default::default()
                },
                &self.text_color,
                TheHorizontalAlign::Center,
                TheVerticalAlign::Center,
            );
        }

        if let Some(color) = self.border_color {
            ctx.draw
                .rect_outline_border(buffer.pixels_mut(), &utuple, stride, &color, 1);
        }

        self.is_dirty = false;
    }

    fn as_icon_view(&mut self) -> Option<&mut dyn TheIconViewTrait> {
        Some(self)
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub trait TheIconViewTrait {
    fn set_rgba_tile(&mut self, tile: TheRGBATile);
    fn step(&mut self);
    fn set_border_color(&mut self, color: Option<RGBA>);
    fn set_text_color(&mut self, color: RGBA);
    /// Set the text to display.
    fn set_text(&mut self, text: Option<String>);
    /// Set the text size.
    fn set_text_size(&mut self, text_size: f32);
    /// Set the alpha blending.
    fn set_alpha_mode(&mut self, alpha_mode: bool);
}

impl TheIconViewTrait for TheIconView {
    fn set_rgba_tile(&mut self, tile: TheRGBATile) {
        self.tile = tile;
        self.is_dirty = true;
        self.index = 0;
    }
    fn step(&mut self) {
        if self.tile.buffer.len() >= 2 {
            self.index += 1;
            if self.index >= self.tile.buffer.len() {
                self.index = 0;
            }
            self.is_dirty = true;
        }
    }
    fn set_border_color(&mut self, color: Option<RGBA>) {
        self.border_color = color;
        self.is_dirty = true;
    }
    fn set_text_color(&mut self, color: RGBA) {
        self.text_color = color;
        self.is_dirty = true;
    }
    fn set_text(&mut self, text: Option<String>) {
        self.text = text;
        self.is_dirty = true;
    }
    fn set_text_size(&mut self, text_size: f32) {
        self.text_size = text_size;
        self.is_dirty = true;
    }
    fn set_alpha_mode(&mut self, alpha_mode: bool) {
        self.alpha_mode = alpha_mode;
    }
}
