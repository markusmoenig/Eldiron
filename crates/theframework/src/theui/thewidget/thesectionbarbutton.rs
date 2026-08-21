use crate::prelude::*;

pub struct TheSectionbarButton {
    id: TheId,
    limiter: TheSizeLimiter,

    state: TheWidgetState,
    status: Option<String>,

    dim: TheDim,
    text: String,
    is_dirty: bool,
}

impl TheWidget for TheSectionbarButton {
    fn new(id: TheId) -> Self
    where
        Self: Sized,
    {
        let mut limiter = TheSizeLimiter::new();
        limiter.set_max_size(Vec2::new(81, 47));

        Self {
            id,
            limiter,

            state: TheWidgetState::None,
            status: None,

            dim: TheDim::zero(),
            text: String::new(),
            is_dirty: false,
        }
    }

    fn id(&self) -> &TheId {
        &self.id
    }

    fn on_event(&mut self, event: &TheEvent, ctx: &mut TheContext) -> bool {
        let mut redraw = false;
        // println!("event ({}): {:?}", self.widget_id.name, event);
        match event {
            TheEvent::MouseDown(_coord) => {
                self.is_dirty = true;
                if self.state != TheWidgetState::Selected {
                    self.state = TheWidgetState::Selected;
                    ctx.ui.send_widget_state_changed(self.id(), self.state);
                }
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

    fn status_text(&self) -> Option<String> {
        self.status.clone()
    }

    /// Sets the status text for the widget.
    fn set_status_text(&mut self, text: &str) {
        self.status = Some(text.to_string());
    }

    fn limiter(&self) -> &TheSizeLimiter {
        &self.limiter
    }

    fn limiter_mut(&mut self) -> &mut TheSizeLimiter {
        &mut self.limiter
    }

    fn state(&self) -> TheWidgetState {
        self.state
    }

    fn set_state(&mut self, state: TheWidgetState) {
        self.state = state;
        self.is_dirty = true;
    }

    fn supports_hover(&mut self) -> bool {
        true
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
        let hovered = self.state != TheWidgetState::Selected && self.id().equals(&ctx.ui.hover);
        let (paint_role, border_role) = if self.state == TheWidgetState::Selected {
            (SectionButtonSelected, ToolListButtonSelectedBorder)
        } else if hovered {
            (SectionButtonHover, ToolListButtonHoverBorder)
        } else {
            (SectionButtonNormal, ToolListButtonNormalBorder)
        };
        let paint = style.theme().paint(paint_role, rect);
        let border = *style.theme().color(border_role);
        let text_color = if self.state == TheWidgetState::Selected {
            *style.theme().color(SectionbarSelectedTextColor)
        } else {
            *style.theme().color(SectionbarNormalTextColor)
        };
        let radius = style.theme().metric(ControlCornerRadius) + 1.0;
        let inner = ThePixelRect::new(
            rect.x.saturating_add(1),
            rect.y.saturating_add(1),
            rect.width.saturating_sub(2),
            rect.height.saturating_sub(2),
        );
        let width = buffer.dim().width.max(0) as usize;
        let height = buffer.dim().height.max(0) as usize;
        if let Ok(mut surface) = TheSurfaceMut::new(buffer.pixels_mut(), width, height) {
            surface.set_clip(rect);
            ctx.painter
                .fill_round_rect(&mut surface, rect, radius, &ThePaint::solid(border));
            ctx.painter
                .fill_round_rect(&mut surface, inner, (radius - 1.0).max(0.0), &paint);
        }

        let stride = buffer.stride();
        let shrinker = TheDimShrinker::zero();
        ctx.draw.text_rect_blend(
            buffer.pixels_mut(),
            &self.dim.to_buffer_shrunk_utuple(&shrinker),
            stride,
            &self.text,
            TheFontSettings {
                size: 15.0,
                ..Default::default()
            },
            &text_color,
            TheHorizontalAlign::Center,
            TheVerticalAlign::Center,
        );

        self.is_dirty = false;
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub trait TheSectionbarButtonTrait {
    fn set_text(&mut self, text: String);
}

impl TheSectionbarButtonTrait for TheSectionbarButton {
    fn set_text(&mut self, text: String) {
        self.text = text;
    }
}
