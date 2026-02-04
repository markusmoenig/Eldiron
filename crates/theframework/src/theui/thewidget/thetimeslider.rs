use crate::prelude::*;

pub struct TheTimeSlider {
    id: TheId,
    limiter: TheSizeLimiter,
    state: TheWidgetState,

    value: TheValue,
    original: TheValue,

    default_value: TheValue,

    status: Option<String>,

    dim: TheDim,
    is_dirty: bool,

    continuous: bool,
    marker: Vec<(TheTime, Vec<String>)>,
    selected_marker: Option<TheTime>,

    text_width: i32,
}

impl TheWidget for TheTimeSlider {
    fn new(id: TheId) -> Self
    where
        Self: Sized,
    {
        let mut limiter = TheSizeLimiter::new();
        limiter.set_max_size(Vec2::new(300, 20));

        Self {
            id,
            limiter,

            state: TheWidgetState::None,

            value: TheValue::Time(TheTime::default()),
            original: TheValue::Float(0.0),

            default_value: TheValue::Float(1.0),

            status: None,

            dim: TheDim::zero(),
            is_dirty: false,

            continuous: false,

            text_width: 40,

            marker: vec![],
            selected_marker: None,
        }
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

    fn value(&self) -> TheValue {
        self.value.clone()
    }

    fn set_value(&mut self, value: TheValue) {
        if value != self.value {
            self.value = value.clone();
            self.default_value = value;
            self.is_dirty = true;
        }
    }

    #[allow(clippy::single_match)]
    fn on_event(&mut self, event: &TheEvent, ctx: &mut TheContext) -> bool {
        let mut redraw = false;
        //println!("event ({}): {:?}", self.widget_id.name, event);
        match event {
            TheEvent::MouseDown(coord) => {
                self.is_dirty = true;
                if self.state != TheWidgetState::Selected {
                    self.state = TheWidgetState::Selected;
                    ctx.ui.send_widget_state_changed(self.id(), self.state);
                }

                ctx.ui.set_focus(self.id());

                let mut offset = coord.x;
                if offset < 0 {
                    offset = 0;
                }

                if offset > self.dim.width - self.text_width {
                    offset = self.dim.width - self.text_width;
                }

                for (time, _) in &self.marker {
                    let marker_offset =
                        time.to_widget_offset((self.dim.width - self.text_width) as u32) as i32;

                    if offset >= marker_offset && offset < marker_offset + 10 {
                        self.selected_marker = Some(*time);
                        ctx.ui
                            .send(TheEvent::TimelineMarkerSelected(self.id.clone(), *time));
                        break;
                    }
                }

                self.value = TheValue::Time(TheTime::from_widget_offset(
                    offset as u32,
                    (self.dim.width - self.text_width) as u32,
                ));

                ctx.ui
                    .send_widget_value_changed(self.id(), self.value.clone());
                redraw = true;
                self.is_dirty = true;
            }
            TheEvent::MouseDragged(coord) => {
                let mut offset = coord.x;
                if offset < 0 {
                    offset = 0;
                }

                if offset > self.dim.width - self.text_width {
                    offset = self.dim.width - self.text_width;
                }

                self.value = TheValue::Time(TheTime::from_widget_offset(
                    offset as u32,
                    (self.dim.width - self.text_width) as u32,
                ));

                if self.continuous {
                    ctx.ui
                        .send_widget_value_changed(self.id(), self.value.clone());
                }
                self.is_dirty = true;
                redraw = true;
            }
            TheEvent::MouseUp(_coord) => {
                self.is_dirty = true;
                if self.state == TheWidgetState::Selected {
                    self.state = TheWidgetState::None;
                }

                if self.value != self.original {
                    ctx.ui
                        .send_widget_value_changed(self.id(), self.value.clone());
                }
                redraw = true;
            }
            TheEvent::Hover(coord) => {
                if self.state != TheWidgetState::Selected && !self.id().equals(&ctx.ui.hover) {
                    self.is_dirty = true;
                    ctx.ui.set_hover(self.id());
                    redraw = true;
                }

                let mut offset = coord.x;
                if offset < 0 {
                    offset = 0;
                }

                if offset > self.dim.width - self.text_width {
                    offset = self.dim.width - self.text_width;
                }

                let mut hovered_marker = false;
                for (time, names) in &self.marker {
                    let marker_offset =
                        time.to_widget_offset((self.dim.width - self.text_width) as u32) as i32;

                    if offset >= marker_offset && offset < marker_offset + 10 {
                        let text = format!("Marker at {}: {}.", time.to_time24(), names.join(", "));
                        ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), text));
                        hovered_marker = true;
                        break;
                    }
                }

                if !hovered_marker {
                    if let Some(status) = &self.status {
                        ctx.ui
                            .send(TheEvent::SetStatusText(TheId::empty(), status.clone()));
                    }
                }
            }
            _ => {}
        }
        redraw
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

        let mut r = self.dim.to_buffer_utuple();

        ctx.draw.rect(
            buffer.pixels_mut(),
            &r,
            stride,
            style.theme().color(TimeSliderBackground),
        );

        ctx.draw.rect_outline(
            buffer.pixels_mut(),
            &r,
            stride,
            style.theme().color(TimeSliderBorder),
        );

        r.2 -= self.text_width as usize;
        let marker_space = r.2 / 23;
        let text_space = r.2 / 11;
        let mut x = r.0;

        for i in 0..=24 {
            if i > 0 {
                let marker_pos = (x, r.1 + r.3 - 4, 2, 2);
                ctx.draw.rect(
                    buffer.pixels_mut(),
                    &marker_pos,
                    stride,
                    style.theme().color(TimeSliderLine),
                );

                if i % 2 == 0 && i < 24 {
                    let text_pos = (x - text_space / 2, r.1, text_space, r.3 - 1);
                    ctx.draw.text_rect_blend(
                        buffer.pixels_mut(),
                        &text_pos,
                        stride,
                        &i.to_string(),
                        TheFontSettings {
                            size: 11.0,
                            ..Default::default()
                        },
                        style.theme().color(TimeSliderText),
                        TheHorizontalAlign::Center,
                        TheVerticalAlign::Top,
                    );
                }
            }
            x += marker_space;
        }

        if let TheValue::Time(time) = &self.value {
            let text_pos = (r.0 + r.2, r.1, self.text_width as usize - 3, r.3);
            ctx.draw.text_rect_blend(
                buffer.pixels_mut(),
                &text_pos,
                stride,
                &time.to_time24(),
                TheFontSettings {
                    size: 11.0,
                    ..Default::default()
                },
                style.theme().color(TimeSliderPosition),
                TheHorizontalAlign::Right,
                TheVerticalAlign::Center,
            );

            let offset = time.to_widget_offset(r.2 as u32) as usize;
            let r = (r.0 + offset, r.1, 2, r.3);
            ctx.draw.rect(
                buffer.pixels_mut(),
                &r,
                stride,
                style.theme().color(TimeSliderPosition),
            );
        }

        for (time, _) in &self.marker {
            let offset = time.to_widget_offset(r.2 as u32) as usize;

            let mut color = style.theme().color(TimeSliderMarker);

            if let Some(selected) = &self.selected_marker {
                if *selected == *time {
                    color = style.theme().color(TimeSliderPosition);
                }
            }

            let mut r = (r.0 + offset, r.1, 2, r.3);
            ctx.draw.rect(buffer.pixels_mut(), &r, stride, color);

            r.2 = 10;
            r.3 = 12;
            ctx.draw.rect(buffer.pixels_mut(), &r, stride, color);
        }

        self.is_dirty = false;
    }

    fn as_time_slider(&mut self) -> Option<&mut dyn TheTimeSliderTrait> {
        Some(self)
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub trait TheTimeSliderTrait: TheWidget {
    fn set_continuous(&mut self, continuous: bool);
    fn clear_marker(&mut self);
    fn add_marker(&mut self, time: TheTime, names: Vec<String>);
    fn remove_marker(&mut self, marker: TheTime);
}

impl TheTimeSliderTrait for TheTimeSlider {
    fn set_continuous(&mut self, continuous: bool) {
        self.continuous = continuous;
    }
    fn clear_marker(&mut self) {
        self.marker = vec![];
        self.selected_marker = None;
        self.is_dirty = true;
    }
    fn add_marker(&mut self, time: TheTime, names: Vec<String>) {
        self.marker.push((time, names));
        self.is_dirty = true;
    }
    fn remove_marker(&mut self, marker: TheTime) {
        self.marker.retain(|(time, _)| *time != marker);
    }
}
