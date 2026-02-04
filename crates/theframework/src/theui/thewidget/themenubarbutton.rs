use crate::prelude::*;

pub struct TheMenubarButton {
    id: TheId,
    limiter: TheSizeLimiter,
    state: TheWidgetState,

    icon_name: String,
    icon_offset: Vec2<i32>,

    fixed_size: Option<Vec2<i32>>,

    status: Option<String>,

    has_state: bool,

    dim: TheDim,
    is_dirty: bool,
}

impl TheWidget for TheMenubarButton {
    fn new(id: TheId) -> Self
    where
        Self: Sized,
    {
        let mut limiter = TheSizeLimiter::new();
        limiter.set_max_size(Vec2::new(35, 35));

        Self {
            id,
            limiter,
            state: TheWidgetState::None,

            icon_name: "".to_string(),
            icon_offset: Vec2::new(0, 0),

            fixed_size: None,

            status: None,

            has_state: false,

            dim: TheDim::zero(),
            is_dirty: false,
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
                if !self.has_state {
                    if !ctx.ui.is_disabled(&self.id.name) {
                        if self.state != TheWidgetState::Clicked {
                            self.state = TheWidgetState::Clicked;
                            ctx.ui.set_focus(self.id());
                            ctx.ui.send_widget_state_changed(self.id(), self.state);
                        }
                        self.is_dirty = true;
                        redraw = true;
                    }
                } else {
                    if !ctx.ui.is_disabled(&self.id.name) {
                        if self.state == TheWidgetState::Clicked {
                            self.state = TheWidgetState::None;
                            ctx.ui.set_focus(self.id());
                            ctx.ui.send_widget_state_changed(self.id(), self.state);
                        } else if self.state == TheWidgetState::None {
                            self.state = TheWidgetState::Clicked;
                            ctx.ui.set_focus(self.id());
                            ctx.ui.send_widget_state_changed(self.id(), self.state);
                        }
                        self.is_dirty = true;
                        redraw = true;
                    }
                }
            }
            TheEvent::Hover(_coord) => {
                if self.state != TheWidgetState::Clicked
                    && !self.id().equals(&ctx.ui.hover)
                    && !ctx.ui.is_disabled(&self.id.name)
                {
                    self.is_dirty = true;
                    ctx.ui.set_hover(self.id());
                    redraw = true;
                }
            }
            TheEvent::MouseUp(_coord) => {
                if !ctx.ui.is_disabled(&self.id.name) {
                    if !self.has_state {
                        if self.state == TheWidgetState::Clicked {
                            self.state = TheWidgetState::None;
                            ctx.ui.clear_focus();
                        }
                        self.is_dirty = true;
                        redraw = true;
                    }
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

    fn draw(
        &mut self,
        buffer: &mut TheRGBABuffer,
        style: &mut Box<dyn TheStyle>,
        ctx: &mut TheContext,
    ) {
        let stride = buffer.stride();
        let mut shrinker = TheDimShrinker::zero();

        if !self.dim().is_valid() {
            return;
        }

        let is_disabled = ctx.ui.is_disabled(&self.id.name);

        if self.state != TheWidgetState::None || self.id().equals(&ctx.ui.hover) && !is_disabled {
            if self.state == TheWidgetState::Clicked {
                ctx.draw.rect_outline_border(
                    buffer.pixels_mut(),
                    &self.dim.to_buffer_shrunk_utuple(&shrinker),
                    stride,
                    style.theme().color(MenubarButtonClickedBorder),
                    1,
                );

                shrinker.shrink(1);

                ctx.draw.rect(
                    buffer.pixels_mut(),
                    &self.dim.to_buffer_shrunk_utuple(&shrinker),
                    stride,
                    style.theme().color(MenubarButtonClicked),
                );
            } else if self.id().equals(&ctx.ui.hover) {
                ctx.draw.rect_outline_border(
                    buffer.pixels_mut(),
                    &self.dim.to_buffer_shrunk_utuple(&shrinker),
                    stride,
                    style.theme().color(MenubarButtonHoverBorder),
                    1,
                );

                shrinker.shrink(1);

                ctx.draw.rect(
                    buffer.pixels_mut(),
                    &self.dim.to_buffer_shrunk_utuple(&shrinker),
                    stride,
                    style.theme().color(MenubarButtonHoverBorder),
                );
            }
        }

        let alpha = if is_disabled { 0.3 } else { 1.0 };

        if let Some(icon) = ctx.ui.icon(&self.icon_name) {
            let utuple = self.dim.to_buffer_shrunk_utuple(&shrinker);

            #[allow(clippy::implicit_saturating_sub)]
            if let Some(fixed_size) = self.fixed_size {
                let r = (
                    ((utuple.0
                        + if utuple.2 > fixed_size.x as usize {
                            utuple.2 - fixed_size.x as usize
                        } else {
                            0
                        } / 2) as i32
                        + self.icon_offset.x) as usize,
                    ((utuple.1
                        + if utuple.3 > fixed_size.y as usize {
                            utuple.3 - fixed_size.y as usize
                        } else {
                            0
                        } / 2) as i32
                        + self.icon_offset.y) as usize,
                    fixed_size.x as usize,
                    fixed_size.y as usize,
                );
                ctx.draw.blend_scale_chunk_alpha(
                    buffer.pixels_mut(),
                    &r,
                    stride,
                    icon.pixels(),
                    &(icon.dim().width as usize, icon.dim().height as usize),
                    alpha,
                );
            } else {
                let r = (
                    ((utuple.0
                        + if utuple.2 > icon.dim().width as usize {
                            utuple.2 - icon.dim().width as usize
                        } else {
                            0
                        } / 2) as i32
                        + self.icon_offset.x) as usize,
                    ((utuple.1
                        + if utuple.3 > icon.dim().height as usize {
                            utuple.3 - icon.dim().height as usize
                        } else {
                            0
                        } / 2) as i32
                        + self.icon_offset.y) as usize,
                    icon.dim().width as usize,
                    icon.dim().height as usize,
                );
                ctx.draw
                    .blend_slice_alpha(buffer.pixels_mut(), icon.pixels(), &r, stride, alpha);
            }
        }

        self.is_dirty = false;
    }

    fn as_menubar_button(&mut self) -> Option<&mut dyn TheMenubarButtonTrait> {
        Some(self)
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub trait TheMenubarButtonTrait {
    fn set_fixed_size(&mut self, size: Vec2<i32>);
    fn set_icon_name(&mut self, text: String);
    fn set_icon_offset(&mut self, offset: Vec2<i32>);
    fn set_has_state(&mut self, has_state: bool);
}

impl TheMenubarButtonTrait for TheMenubarButton {
    fn set_fixed_size(&mut self, size: Vec2<i32>) {
        self.fixed_size = Some(size);
        self.is_dirty = true;
    }
    fn set_icon_name(&mut self, text: String) {
        self.icon_name = text;
        self.is_dirty = true;
    }
    fn set_icon_offset(&mut self, offset: Vec2<i32>) {
        self.icon_offset = offset;
    }
    fn set_has_state(&mut self, has_state: bool) {
        self.has_state = has_state;
    }
}
