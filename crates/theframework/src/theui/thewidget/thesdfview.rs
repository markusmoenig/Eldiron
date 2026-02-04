use crate::prelude::*;

pub struct TheSDFView {
    id: TheId,
    limiter: TheSizeLimiter,
    state: TheWidgetState,

    is_disabled: bool,

    status_text: FxHashMap<usize, String>,

    text: String,
    text_size: f32,

    dim: TheDim,
    is_dirty: bool,

    canvas: TheSDFCanvas,
}

impl TheWidget for TheSDFView {
    fn new(id: TheId) -> Self
    where
        Self: Sized,
    {
        let limiter = TheSizeLimiter::new();

        Self {
            id,
            limiter,
            state: TheWidgetState::None,

            status_text: FxHashMap::default(),

            text: "".to_string(),
            text_size: 13.0,

            dim: TheDim::zero(),
            is_dirty: false,
            is_disabled: false,

            canvas: TheSDFCanvas::new(),
        }
    }

    fn id(&self) -> &TheId {
        &self.id
    }

    fn status_text(&self) -> Option<String> {
        Some("".to_string())
    }

    #[allow(clippy::single_match)]
    fn on_event(&mut self, event: &TheEvent, ctx: &mut TheContext) -> bool {
        if self.is_disabled {
            return false;
        }
        let mut redraw = false;
        //println!("event ({}): {:?}", self.id.name, event);
        match event {
            TheEvent::MouseDown(coord) => {
                if self.state != TheWidgetState::Clicked {
                    self.state = TheWidgetState::Clicked;
                    ctx.ui.set_focus(self.id());
                    ctx.ui
                        .send_widget_state_changed(self.id(), TheWidgetState::Clicked);
                }
                if let Some(index) = self
                    .canvas
                    .index_at(Vec2::new(coord.x as f32, coord.y as f32))
                {
                    if Some(index) != self.canvas.selected {
                        self.canvas.selected = Some(index);
                        self.is_dirty = true;
                        redraw = true;
                        ctx.ui
                            .send(TheEvent::SDFIndexChanged(self.id.clone(), index as u32));
                    }
                }
            }
            TheEvent::Hover(coord) => {
                if self.state != TheWidgetState::Clicked && !self.id().equals(&ctx.ui.hover) {
                    self.is_dirty = true;
                    ctx.ui.set_hover(self.id());
                    redraw = true;
                }
                if let Some(index) = self
                    .canvas
                    .index_at(Vec2::new(coord.x as f32, coord.y as f32))
                {
                    if Some(index) != self.canvas.hover {
                        self.canvas.hover = Some(index);
                        self.is_dirty = true;
                        redraw = true;
                        if let Some(status) = self.status_text.get(&index) {
                            ctx.ui
                                .send(TheEvent::SetStatusText(self.id.clone(), status.clone()));
                        }
                    }
                } else if self.canvas.hover.is_some() {
                    self.canvas.hover = None;
                    self.is_dirty = true;
                    redraw = true;
                    ctx.ui
                        .send(TheEvent::SetStatusText(self.id.clone(), "".to_string()));
                }
            }
            TheEvent::LostHover(_coord) => {
                self.canvas.hover = None;
                self.is_dirty = true;
                redraw = true;
                ctx.ui
                    .send(TheEvent::SetStatusText(self.id.clone(), "".to_string()));
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
        if !self.text.is_empty() {
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
        _style: &mut Box<dyn TheStyle>,
        _ctx: &mut TheContext,
    ) {
        if !self.dim().is_valid() {
            return;
        }

        let mut b = TheRGBABuffer::new(TheDim::new(0, 0, self.dim.width, self.dim.height));

        self.canvas.render(&mut b);
        buffer.copy_into(self.dim.buffer_x, self.dim.buffer_y, &b);

        self.is_dirty = false;
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub trait TheSDFViewTrait {
    fn set_canvas(&mut self, canvas: TheSDFCanvas);
    fn set_status(&mut self, index: usize, text: String);
}

impl TheSDFViewTrait for TheSDFView {
    fn set_canvas(&mut self, canvas: TheSDFCanvas) {
        self.canvas = canvas;
    }
    fn set_status(&mut self, index: usize, text: String) {
        self.status_text.insert(index, text);
    }
}
