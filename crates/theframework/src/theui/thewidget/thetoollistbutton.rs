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
                ctx.ui
                    .send_widget_state_changed(self.id(), TheWidgetState::Clicked);
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
        let hovered = self.state == TheWidgetState::None && self.id().equals(&ctx.ui.hover);
        let (paint_role, border_role) = if self.state == TheWidgetState::Selected {
            (ControlPressed, ToolListButtonSelectedBorder)
        } else if hovered {
            (ControlHover, ToolListButtonHoverBorder)
        } else {
            (ControlNormal, ToolListButtonNormalBorder)
        };
        let paint = style.theme().paint(paint_role, rect);
        let border = *style.theme().color(border_role);
        let radius = style.theme().metric(ControlCornerRadius);
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
        if let Some(icon) = ctx.ui.icon(&self.icon_name) {
            let icon_width = icon.dim().width.max(0);
            let icon_height = icon.dim().height.max(0);
            if icon_width <= rect.width && icon_height <= rect.height {
                let icon_x = rect.x + (rect.width - icon_width) / 2;
                let icon_y = rect.y + (rect.height - icon_height) / 2;
                if icon_x >= 0
                    && icon_y >= 0
                    && icon_x + icon_width <= width as i32
                    && icon_y + icon_height <= height as i32
                {
                    ctx.draw.blend_slice(
                        buffer.pixels_mut(),
                        icon.pixels(),
                        &(
                            icon_x as usize,
                            icon_y as usize,
                            icon_width as usize,
                            icon_height as usize,
                        ),
                        stride,
                    );
                }
            }
        }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn procedural_chrome_clips_when_widget_extends_outside_buffer() {
        let mut button = TheToolListButton::new(TheId::named("Clipped Tool"));
        let mut ctx = TheContext::new(8, 8, 1.0);
        button.set_dim(TheDim::rect(-5, -7, 46, 43), &mut ctx);
        let mut style: Box<dyn TheStyle> = Box::new(TheClassicStyle::with_theme(Box::new(
            TheBlackBlueTheme::new(),
        )));
        let mut buffer = TheRGBABuffer::new(TheDim::sized(8, 8));

        button.draw(&mut buffer, &mut style, &mut ctx);

        assert!(buffer.pixels().chunks_exact(4).any(|pixel| pixel[3] != 0));
    }
}
