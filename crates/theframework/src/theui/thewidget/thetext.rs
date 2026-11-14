use crate::prelude::*;

pub struct TheText {
    #[cfg(feature = "text")]
    paint_revision: Uuid,
    id: TheId,

    limiter: TheSizeLimiter,

    dim: TheDim,
    text: String,
    text_size: f32,
    text_color: Option<RGBA>,
    vertical_offset: i32,

    fixed_size_text: String,

    is_dirty: bool,
}

impl TheText {
    fn invalidate_retained_paint(&mut self) {
        self.is_dirty = true;
        #[cfg(feature = "text")]
        {
            self.paint_revision = Uuid::new_v4();
        }
    }
}

impl TheWidget for TheText {
    #[cfg(feature = "text")]
    fn retained_paint_revision(&self) -> Option<Uuid> {
        (self.text.len() <= 256).then_some(self.paint_revision)
    }
    #[cfg(feature = "text")]
    fn paint(
        &mut self,
        p: &mut crate::theui::paint::WidgetPainter<'_>,
    ) -> crate::theui::paint::UiPaintResult {
        use crate::theui::paint::bounds;
        let mut r = bounds(&self.dim);
        r.y += 1. + self.vertical_offset as f32;
        r.height -= 1.;
        p.label(
            r,
            &self.text,
            self.text_size,
            self.text_color
                .unwrap_or(*p.theme.color(SectionbarNormalTextColor)),
            TheHorizontalAlign::Left,
            TheVerticalAlign::Center,
        );
        self.is_dirty = false;
        Ok(())
    }

    fn new(id: TheId) -> Self
    where
        Self: Sized,
    {
        let mut limiter = TheSizeLimiter::new();
        limiter.set_max_height(21);

        Self {
            #[cfg(feature = "text")]
            paint_revision: Uuid::new_v4(),
            id,
            limiter,

            dim: TheDim::zero(),
            text: "".to_string(),
            text_size: 13.0,
            text_color: None,
            vertical_offset: 0,

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
                let size = ctx.measure_text(
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
            let size = ctx.measure_text(
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
        style: &mut Box<dyn TheStyle>,
        ctx: &mut TheContext,
    ) {
        if !self.dim().is_valid() {
            return;
        }

        let stride = buffer.stride();

        let mut shrinker = TheDimShrinker::zero();
        // Preserve the text rectangle's height while allowing individual labels
        // to tune their visual baseline inside custom toolbars.
        shrinker.shrink_by(0, 1 + self.vertical_offset, 0, -self.vertical_offset);

        ctx.draw.text_rect_blend(
            buffer.draw_target(),
            &self.dim.to_buffer_shrunk_utuple(&shrinker),
            stride,
            &self.text,
            TheFontSettings {
                size: self.text_size,
                ..Default::default()
            },
            &self
                .text_color
                .unwrap_or(*style.theme().color(SectionbarNormalTextColor)),
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
    /// Moves the rendered text vertically without changing its layout size.
    fn set_vertical_offset(&mut self, offset: i32);
    /// Set fixed size text.
    fn set_fixed_size_text(&mut self, fixed_size_text: String);
}

impl TheTextTrait for TheText {
    fn set_text(&mut self, text: String) {
        if self.text == text {
            return;
        }
        self.invalidate_retained_paint();
        self.text = text;
        self.is_dirty = true;
    }
    fn set_text_size(&mut self, text_size: f32) {
        if self.text_size == text_size {
            return;
        }
        self.invalidate_retained_paint();
        self.text_size = text_size;
        self.is_dirty = true;
    }
    fn set_text_color(&mut self, color: RGBA) {
        if self.text_color == Some(color) {
            return;
        }
        self.invalidate_retained_paint();
        self.text_color = Some(color);
        self.is_dirty = true;
    }
    fn set_vertical_offset(&mut self, offset: i32) {
        if self.vertical_offset == offset {
            return;
        }
        self.invalidate_retained_paint();
        self.vertical_offset = offset;
        self.is_dirty = true;
    }
    fn set_fixed_size_text(&mut self, fixed_size_text: String) {
        if self.fixed_size_text == fixed_size_text {
            return;
        }
        self.invalidate_retained_paint();
        self.fixed_size_text = fixed_size_text;
        self.is_dirty = true;
    }
}
