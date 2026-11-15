use crate::prelude::*;
use crate::thecontext::TheCursorIcon;

pub struct TheColorButton {
    #[cfg(feature = "text")]
    paint_revision: Uuid,
    id: TheId,
    limiter: TheSizeLimiter,
    state: TheWidgetState,

    status: Option<String>,

    dim: TheDim,
    color: RGBA,
    is_dirty: bool,
    cursor_icon: Option<TheCursorIcon>,
}

impl TheColorButton {
    fn invalidate_retained_paint(&mut self) {
        self.is_dirty = true;
        #[cfg(feature = "text")]
        {
            self.paint_revision = Uuid::new_v4();
        }
    }
}

impl TheWidget for TheColorButton {
    #[cfg(feature = "text")]
    fn retained_paint_revision(&self) -> Option<Uuid> {
        Some(self.paint_revision)
    }

    #[cfg(feature = "text")]
    fn paint(
        &mut self,
        p: &mut crate::theui::paint::WidgetPainter<'_>,
    ) -> crate::theui::paint::UiPaintResult {
        use crate::theui::paint::bounds;
        let r = bounds(&self.dim);
        use crate::theui::paint::rgba;
        p.list.rect(
            r,
            0.,
            rgba(if self.state == TheWidgetState::None {
                [80, 80, 80, 255]
            } else {
                BLACK
            }),
        );
        p.list.rect(r.inset(1.), 0., rgba(self.color));
        self.is_dirty = false;
        Ok(())
    }
    fn new(id: TheId) -> Self
    where
        Self: Sized,
    {
        let mut limiter = TheSizeLimiter::new();
        limiter.set_max_size(Vec2::new(20, 20));
        Self {
            #[cfg(feature = "text")]
            paint_revision: Uuid::new_v4(),
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
                self.invalidate_retained_paint();
                ctx.ui.send(TheEvent::ColorButtonClicked(self.id.clone()));
                redraw = true;
            }
            TheEvent::MouseUp(_coord) => {
                self.state = TheWidgetState::None;
                ctx.ui.send_widget_state_changed(self.id(), self.state);
                self.invalidate_retained_paint();
                redraw = true;
            }
            TheEvent::Hover(_coord) => {
                if !self.id().equals(&ctx.ui.hover) {
                    self.invalidate_retained_paint();
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
            self.invalidate_retained_paint();
        }
    }

    fn set_state(&mut self, state: TheWidgetState) {
        self.state = state;
        self.invalidate_retained_paint();
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
        if redraw {
            self.invalidate_retained_paint();
        } else {
            self.is_dirty = false;
        }
    }

    fn supports_hover(&mut self) -> bool {
        true
    }

    fn set_value(&mut self, value: TheValue) {
        if let TheValue::ColorObject(color) = value {
            self.color = color.to_u8_array();
            self.invalidate_retained_paint();
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
                buffer.draw_target(),
                &self.dim.to_buffer_shrunk_utuple(&shrinker),
                stride,
                &[80, 80, 80, 255],
                1,
            );
        } else {
            ctx.draw.rect_outline_border(
                buffer.draw_target(),
                &self.dim.to_buffer_shrunk_utuple(&shrinker),
                stride,
                &BLACK,
                1,
            );
        }

        shrinker.shrink(1);
        ctx.draw.rect(
            buffer.draw_target(),
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
