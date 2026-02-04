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

        let stride = buffer.stride();
        let shrinker = TheDimShrinker::zero();

        let utuple: (usize, usize, usize, usize) = self.dim.to_buffer_utuple();

        let mut icon_name = if self.state == TheWidgetState::Selected {
            "dark_sectionbarbutton_selected".to_string()
        } else {
            "dark_sectionbarbutton_normal".to_string()
        };

        if self.state != TheWidgetState::Selected && self.id().equals(&ctx.ui.hover) {
            icon_name = "dark_sectionbarbutton_hover".to_string()
        }

        let text_color = if self.state == TheWidgetState::Selected {
            style.theme().color(SectionbarSelectedTextColor)
        } else {
            style.theme().color(SectionbarNormalTextColor)
        };

        if let Some(icon) = ctx.ui.icon(&icon_name) {
            let r = (
                utuple.0,
                utuple.1,
                icon.dim().width as usize,
                icon.dim().height as usize,
            );
            ctx.draw
                .blend_slice(buffer.pixels_mut(), icon.pixels(), &r, stride);
        }

        ctx.draw.text_rect_blend(
            buffer.pixels_mut(),
            &self.dim.to_buffer_shrunk_utuple(&shrinker),
            stride,
            &self.text,
            TheFontSettings {
                size: 15.0,
                ..Default::default()
            },
            text_color,
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
