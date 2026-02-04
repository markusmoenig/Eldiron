use crate::prelude::*;

pub struct TheTraybarButton {
    id: TheId,
    limiter: TheSizeLimiter,
    state: TheWidgetState,

    is_disabled: bool,

    icon_name: String,
    icon: Option<TheRGBABuffer>,
    icon_offset: Vec2<i32>,

    status: Option<String>,

    text: String,
    text_size: f32,

    dim: TheDim,
    is_dirty: bool,

    custom_color: Option<TheColor>,
    context_menu: Option<TheContextMenu>,

    fixed_size: bool,
}

impl TheWidget for TheTraybarButton {
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

            icon_name: "".to_string(),
            icon_offset: Vec2::new(0, 0),
            icon: None,

            status: None,

            text: "".to_string(),
            text_size: 13.0,

            dim: TheDim::zero(),
            is_dirty: false,
            is_disabled: false,

            custom_color: None,
            context_menu: None,
            fixed_size: false,
        }
    }

    fn id(&self) -> &TheId {
        &self.id
    }

    fn status_text(&self) -> Option<String> {
        self.status.clone()
    }

    /// Sets the status text for the widget.
    fn set_status_text(&mut self, text: &str) {
        self.status = Some(text.to_string());
    }

    fn set_context_menu(&mut self, menu: Option<TheContextMenu>) {
        self.context_menu = menu;
    }

    #[allow(clippy::single_match)]
    fn on_event(&mut self, event: &TheEvent, ctx: &mut TheContext) -> bool {
        if self.is_disabled {
            return false;
        }
        let mut redraw = false;
        //println!("event ({}): {:?}", self.id.name, event);
        match event {
            TheEvent::MouseDown(_coord) => {
                if self.state != TheWidgetState::Clicked {
                    //self.state = TheWidgetState::Clicked;
                    ctx.ui.set_focus(self.id());
                    ctx.ui
                        .send_widget_state_changed(self.id(), TheWidgetState::Clicked);
                }

                if let Some(context_menu) = &self.context_menu {
                    ctx.ui.send(TheEvent::ShowContextMenu(
                        self.id().clone(),
                        Vec2::new(self.dim.x, self.dim.y + self.dim.height),
                        context_menu.clone(),
                    ));

                    ctx.ui.clear_focus();
                    ctx.ui.clear_hover();
                }

                self.is_dirty = true;
                redraw = true;
            }
            TheEvent::Hover(_coord) => {
                if self.state != TheWidgetState::Clicked && !self.id().equals(&ctx.ui.hover) {
                    self.is_dirty = true;
                    ctx.ui.set_hover(self.id());
                    redraw = true;
                }
            }
            TheEvent::MouseUp(_coord) => {
                if self.state == TheWidgetState::Clicked {
                    self.state = TheWidgetState::None;
                    ctx.ui.clear_focus();
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

    fn set_dim(&mut self, dim: TheDim, _ctx: &mut TheContext) {
        if self.dim != dim {
            self.dim = dim;
            self.is_dirty = true;
        }
    }

    fn disabled(&self) -> bool {
        self.is_disabled
    }

    fn set_disabled(&mut self, disabled: bool) {
        if disabled != self.is_disabled {
            self.is_disabled = disabled;
            self.is_dirty = true;
            self.state = TheWidgetState::None;
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

    fn set_needs_redraw(&mut self, redraw: bool) {
        self.is_dirty = redraw;
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

    fn calculate_size(&mut self, ctx: &mut TheContext) {
        if !self.text.is_empty() && !self.fixed_size {
            let size = ctx.draw.get_text_size(
                &self.text,
                &TheFontSettings {
                    size: self.text_size,
                    ..Default::default()
                },
            );
            self.limiter_mut()
                .set_max_width((size.0 as f32).ceil() as i32 + 15);
        }
    }

    fn draw(
        &mut self,
        buffer: &mut TheRGBABuffer,
        style: &mut Box<dyn TheStyle>,
        ctx: &mut TheContext,
    ) {
        let stride = buffer.stride();
        let mut shrinker: TheDimShrinker = TheDimShrinker::zero();

        if !self.dim().is_valid() {
            return;
        }

        if self.is_disabled {
            ctx.draw.rect_outline_border(
                buffer.pixels_mut(),
                &self.dim.to_buffer_shrunk_utuple(&shrinker),
                stride,
                style.theme().color(TraybarButtonDisabledBorder),
                1,
            );

            shrinker.shrink(1);

            ctx.draw.rect(
                buffer.pixels_mut(),
                &self.dim.to_buffer_shrunk_utuple(&shrinker),
                stride,
                style.theme().color(TraybarButtonDisabledBackground),
            );
        }

        if !self.is_disabled
            && self.state == TheWidgetState::None
            && !self.id().equals(&ctx.ui.hover)
        {
            ctx.draw.rect_outline_border(
                buffer.pixels_mut(),
                &self.dim.to_buffer_shrunk_utuple(&shrinker),
                stride,
                style.theme().color(TraybarButtonNormalBorder),
                1,
            );

            shrinker.shrink(1);

            ctx.draw.rect(
                buffer.pixels_mut(),
                &self.dim.to_buffer_shrunk_utuple(&shrinker),
                stride,
                style.theme().color(TraybarButtonNormal),
            );
        }

        if !self.is_disabled && self.state != TheWidgetState::None
            || self.id().equals(&ctx.ui.hover)
        {
            if self.state == TheWidgetState::Clicked {
                ctx.draw.rect_outline_border(
                    buffer.pixels_mut(),
                    &self.dim.to_buffer_shrunk_utuple(&shrinker),
                    stride,
                    style.theme().color(TraybarButtonClickedBorder),
                    1,
                );

                shrinker.shrink(1);

                ctx.draw.rect(
                    buffer.pixels_mut(),
                    &self.dim.to_buffer_shrunk_utuple(&shrinker),
                    stride,
                    style.theme().color(TraybarButtonClicked),
                );
            } else if self.id().equals(&ctx.ui.hover) {
                ctx.draw.rect_outline_border(
                    buffer.pixels_mut(),
                    &self.dim.to_buffer_shrunk_utuple(&shrinker),
                    stride,
                    style.theme().color(TraybarButtonHover),
                    1,
                );

                shrinker.shrink(1);

                ctx.draw.rect(
                    buffer.pixels_mut(),
                    &self.dim.to_buffer_shrunk_utuple(&shrinker),
                    stride,
                    style.theme().color(TraybarButtonHoverBorder),
                );
            }
        }

        if let Some(icon) = &self.icon {
            let utuple = self.dim.to_buffer_shrunk_utuple(&shrinker);
            let r = (
                ((utuple.0 + (utuple.2 - icon.dim().width as usize) / 2) as i32
                    + self.icon_offset.x) as usize,
                ((utuple.1 + (utuple.3 - icon.dim().height as usize) / 2) as i32
                    + self.icon_offset.y) as usize,
                icon.dim().width as usize,
                icon.dim().height as usize,
            );
            ctx.draw
                .blend_slice(buffer.pixels_mut(), icon.pixels(), &r, stride);
        } else if let Some(icon) = ctx.ui.icon(&self.icon_name) {
            let utuple = self.dim.to_buffer_shrunk_utuple(&shrinker);
            let r = (
                ((utuple.0 + (utuple.2 - icon.dim().width as usize) / 2) as i32
                    + self.icon_offset.x) as usize,
                ((utuple.1 + (utuple.3 - icon.dim().height as usize) / 2) as i32
                    + self.icon_offset.y) as usize,
                icon.dim().width as usize,
                icon.dim().height as usize,
            );
            ctx.draw
                .blend_slice(buffer.pixels_mut(), icon.pixels(), &r, stride);
        }

        if !self.text.is_empty() {
            let color = if let Some(custom) = &self.custom_color {
                &custom.to_u8_array()
            } else {
                &WHITE
            };
            ctx.draw.text_rect_blend(
                buffer.pixels_mut(),
                &self.dim.to_buffer_shrunk_utuple(&shrinker),
                stride,
                &self.text,
                TheFontSettings {
                    size: self.text_size,
                    ..Default::default()
                },
                color,
                TheHorizontalAlign::Center,
                TheVerticalAlign::Center,
            );
        }

        self.is_dirty = false;
    }

    fn set_value(&mut self, value: TheValue) {
        if let TheValue::Text(text) = value {
            self.text = text;
            self.is_dirty = true;
        }
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub trait TheTraybarButtonTrait {
    fn set_icon_name(&mut self, text: String);
    fn set_icon_offset(&mut self, offset: Vec2<i32>);
    fn set_text(&mut self, text: String);
    fn set_icon(&mut self, icon: TheRGBABuffer);
    fn set_fixed_size(&mut self, fixed_size: bool);
    fn set_custom_color(&mut self, color: Option<TheColor>);
}

impl TheTraybarButtonTrait for TheTraybarButton {
    fn set_icon_name(&mut self, text: String) {
        self.icon_name = text;
    }
    fn set_icon(&mut self, icon: TheRGBABuffer) {
        self.icon = Some(icon);
    }
    fn set_icon_offset(&mut self, offset: Vec2<i32>) {
        self.icon_offset = offset;
    }
    fn set_text(&mut self, text: String) {
        self.text = text;
    }
    fn set_fixed_size(&mut self, fixed_size: bool) {
        self.fixed_size = fixed_size;
    }
    fn set_custom_color(&mut self, color: Option<TheColor>) {
        self.custom_color = color;
    }
}
