use crate::prelude::*;

pub struct TheToolListButton {
    id: TheId,
    limiter: TheSizeLimiter,

    state: TheWidgetState,
    status: Option<String>,

    dim: TheDim,
    icon_name: String,
    is_dirty: bool,
}

impl TheWidget for TheToolListButton {
    fn new(id: TheId) -> Self
    where
        Self: Sized,
    {
        let mut limiter = TheSizeLimiter::new();
        limiter.set_max_size(Vec2::new(46, 43));

        Self {
            id,
            limiter,

            state: TheWidgetState::None,
            status: None,

            dim: TheDim::zero(),
            icon_name: String::new(),
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

        let mut shrinker = TheDimShrinker::zero();
        let utuple = self.dim.to_buffer_shrunk_utuple(&shrinker);

        if self.state == TheWidgetState::None && self.id().equals(&ctx.ui.hover) {
            ctx.draw.rect_outline_border(
                buffer.pixels_mut(),
                &utuple,
                stride,
                style.theme().color(ToolListButtonHoverBorder),
                1,
            );
            shrinker.shrink(1);
            let utuple = self.dim.to_buffer_shrunk_utuple(&shrinker);
            ctx.draw.rect(
                buffer.pixels_mut(),
                &utuple,
                stride,
                style.theme().color(ToolListButtonHoverBackground),
            );
        } else if self.state == TheWidgetState::None {
            ctx.draw.rect_outline_border(
                buffer.pixels_mut(),
                &utuple,
                stride,
                style.theme().color(ToolListButtonNormalBorder),
                1,
            );
        } else if self.state == TheWidgetState::Selected {
            ctx.draw.rect_outline_border(
                buffer.pixels_mut(),
                &utuple,
                stride,
                style.theme().color(ToolListButtonSelectedBorder),
                1,
            );
            shrinker.shrink(1);
            let utuple = self.dim.to_buffer_shrunk_utuple(&shrinker);
            ctx.draw.rect(
                buffer.pixels_mut(),
                &utuple,
                stride,
                style.theme().color(ToolListButtonSelectedBackground),
            );
        };

        if let Some(icon) = ctx.ui.icon(&self.icon_name) {
            ctx.draw.blend_slice(
                buffer.pixels_mut(),
                icon.pixels(),
                &(
                    utuple.0 + (utuple.2 - icon.dim().width as usize) / 2,
                    utuple.1 + (utuple.3 - icon.dim().height as usize) / 2,
                    icon.dim().width as usize,
                    icon.dim().height as usize,
                ),
                stride,
            );
        }

        /*
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

        if let Some(font) = &ctx.ui.font {
            ctx.draw.text_rect_blend(
                buffer.pixels_mut(),
                &self.dim.to_buffer_shrunk_utuple(&shrinker),
                stride,
                font,
                15.0,
                &self.text,
                text_color,
                TheHorizontalAlign::Center,
                TheVerticalAlign::Center,
            );
            }*/

        self.is_dirty = false;
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub trait TheToolListButtonTrait {
    fn set_icon_name(&mut self, text: String);
}

impl TheToolListButtonTrait for TheToolListButton {
    fn set_icon_name(&mut self, icon_name: String) {
        self.icon_name = icon_name;
        self.is_dirty = true;
    }
}
