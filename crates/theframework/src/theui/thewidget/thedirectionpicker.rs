use crate::prelude::*;

pub struct TheDirectionPicker {
    id: TheId,
    limiter: TheSizeLimiter,
    state: TheWidgetState,

    dim: TheDim,
    is_dirty: bool,

    value: TheValue,
}

impl TheWidget for TheDirectionPicker {
    fn new(id: TheId) -> Self
    where
        Self: Sized,
    {
        let mut limiter = TheSizeLimiter::new();
        limiter.set_max_size(Vec2::new(200, 200));

        Self {
            id,
            limiter,
            state: TheWidgetState::None,

            dim: TheDim::zero(),
            is_dirty: false,

            value: TheValue::Direction(Vec3::new(0.0, 0.0, -1.0)),
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
            TheEvent::MouseDown(coord) => {
                if self.state != TheWidgetState::Selected {
                    self.state = TheWidgetState::Selected;
                    ctx.ui.send_widget_state_changed(self.id(), self.state);
                }
                ctx.ui.set_focus(self.id());

                if let TheValue::Direction(_) = self.value {
                    let v = Vec2::new(
                        (coord.x - self.dim().width / 2) as f32,
                        (coord.y - self.dim().height / 2) as f32,
                    );
                    let norm = v.normalized();

                    let value = Vec3::new(norm.x, 0.0, norm.y);
                    self.value = TheValue::Direction(value);
                    ctx.ui
                        .send(TheEvent::ValueChanged(self.id.clone(), self.value.clone()));
                }

                self.is_dirty = true;
                redraw = true;
            }
            TheEvent::MouseDragged(coord) => {
                if let TheValue::Direction(_) = self.value {
                    let v = Vec2::new(
                        (coord.x - self.dim().width / 2) as f32,
                        (coord.y - self.dim().height / 2) as f32,
                    );
                    let norm = v.normalized();

                    let value = Vec3::new(norm.x, 0.0, norm.y);
                    self.value = TheValue::Direction(value);
                    ctx.ui
                        .send(TheEvent::ValueChanged(self.id.clone(), self.value.clone()));
                }

                self.is_dirty = true;
                redraw = true;
            }
            TheEvent::MouseUp(_coord) => {
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

    fn set_state(&mut self, state: TheWidgetState) {
        self.state = state;
        self.is_dirty = true;
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

        let ut = self.dim.to_buffer_utuple();

        ctx.draw.circle(
            buffer.pixels_mut(),
            &ut,
            stride,
            &[128, 128, 128, 255],
            90.0,
        );

        if let TheValue::Direction(value) = self.value {
            let cx = buffer.dim().width / 2;
            let cy = buffer.dim().height / 2 - 6;

            buffer.draw_line(
                cx,
                cy,
                cx + (value.x * 90.0) as i32,
                cy + (value.z * 90.0) as i32,
                BLACK,
            );
        }

        self.is_dirty = false;
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn value(&self) -> TheValue {
        self.value.clone()
    }

    fn set_value(&mut self, value: TheValue) {
        if let TheValue::Direction(_) = value {
            self.value = value;
            self.is_dirty = true;
        }
    }
}
