use crate::prelude::*;

pub struct TheHorizontalScrollbar {
    id: TheId,
    limiter: TheSizeLimiter,

    state: TheWidgetState,

    scroll_offset: i32,
    total_width: i32,

    mouse_down_coord: Vec2<i32>,

    dim: TheDim,
    is_dirty: bool,
}

impl TheWidget for TheHorizontalScrollbar {
    fn new(id: TheId) -> Self
    where
        Self: Sized,
    {
        let mut limiter = TheSizeLimiter::new();
        limiter.set_max_height(20);

        Self {
            id,
            limiter,

            state: TheWidgetState::None,

            scroll_offset: 0,
            total_width: 0,

            mouse_down_coord: Vec2::zero(),

            dim: TheDim::zero(),
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
            TheEvent::MouseDown(coord) => {
                let dim = TheDim::new(
                    self.scrollbar_position() as i32,
                    0,
                    self.scrollbar_thumb_width(),
                    self.dim.height,
                );
                if dim.contains(*coord) {
                    self.is_dirty = true;
                    if self.state != TheWidgetState::Clicked {
                        self.state = TheWidgetState::Clicked;
                        ctx.ui.send_widget_state_changed(self.id(), self.state);
                        ctx.ui.set_focus(self.id());
                        self.mouse_down_coord = *coord;
                    }
                } else {
                    self.is_dirty = true;
                    self.scroll_from_track_click(coord.x);
                }
                redraw = true;
            }
            TheEvent::MouseDragged(coord) => {
                if self.state == TheWidgetState::Clicked {
                    self.is_dirty = true;
                    redraw = true;
                    let d = coord - self.mouse_down_coord;
                    self.adjust_scroll_from_thumb_delta(d.x);
                    self.mouse_down_coord = *coord;
                }
            }
            TheEvent::MouseUp(_coord) => {
                self.is_dirty = true;
                if self.state == TheWidgetState::Clicked {
                    self.state = TheWidgetState::None;
                    ctx.ui.send_widget_state_changed(self.id(), self.state);
                }
                redraw = true;
            }
            TheEvent::Hover(coord) => {
                if self.state != TheWidgetState::Clicked {
                    let dim = TheDim::new(
                        self.scrollbar_position() as i32,
                        0,
                        self.scrollbar_thumb_width(),
                        self.dim.height,
                    );
                    if dim.contains(*coord) {
                        if !self.id().equals(&ctx.ui.hover) {
                            self.is_dirty = true;
                            ctx.ui.set_hover(self.id());
                            redraw = true;
                        }
                    } else {
                        self.is_dirty = true;
                        ctx.ui.clear_hover();
                        redraw = true;
                    }
                }
            }
            TheEvent::ScrollBy(_id, delta) => {
                self.adjust_scroll_from_thumb_delta(delta.x);
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
        if !self.dim().is_valid() {
            return;
        }

        let stride = buffer.stride();
        //let mut shrinker = TheDimShrinker::zero();

        let utuple: (usize, usize, usize, usize) = self.dim.to_buffer_utuple();

        ctx.draw.rect(
            buffer.pixels_mut(),
            &utuple,
            stride,
            style.theme().color(ScrollbarBackground),
        );

        let mut icon_name = if self.state == TheWidgetState::Clicked {
            "dark_horizontal_scrollbar_clicked_".to_string()
        } else {
            "dark_horizontal_scrollbar_normal_".to_string()
        };

        if self.state != TheWidgetState::Clicked && self.id().equals(&ctx.ui.hover) {
            icon_name = "dark_horizontal_scrollbar_hover_".to_string()
        }

        let mut scroll_bar_width = self.scrollbar_thumb_width();
        let mut offset = self.scrollbar_position() as usize;

        if scroll_bar_width > self.dim.width {
            offset = 0;
            scroll_bar_width = self.dim.width;
        }

        if scroll_bar_width >= 5 {
            if let Some(icon) = ctx.ui.icon(&(icon_name.clone() + "left")) {
                let r = (
                    utuple.0 + offset,
                    utuple.1,
                    icon.dim().width as usize,
                    icon.dim().height as usize,
                );
                ctx.draw
                    .blend_slice(buffer.pixels_mut(), icon.pixels(), &r, stride);
            }
        }

        if scroll_bar_width > 10 {
            if let Some(icon) = ctx.ui.icon(&(icon_name.clone() + "middle")) {
                let mut r = (
                    utuple.0 + offset + 5,
                    utuple.1,
                    icon.dim().width as usize,
                    icon.dim().height as usize,
                );
                for _ in 0..scroll_bar_width - 10 {
                    ctx.draw
                        .copy_slice(buffer.pixels_mut(), icon.pixels(), &r, stride);
                    r.0 += 1;
                }
            }
        }

        if scroll_bar_width >= 10 {
            if let Some(icon) = ctx.ui.icon(&(icon_name + "right")) {
                let r = (
                    utuple.0 + offset + scroll_bar_width as usize - 5,
                    utuple.1,
                    icon.dim().width as usize,
                    icon.dim().height as usize,
                );
                ctx.draw
                    .blend_slice(buffer.pixels_mut(), icon.pixels(), &r, stride);
            }
        }

        self.is_dirty = false;
    }

    fn as_horizontal_scrollbar(&mut self) -> Option<&mut dyn TheHorizontalScrollbarTrait> {
        Some(self)
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub trait TheHorizontalScrollbarTrait {
    /// Returns the total height of the content.
    fn total_width(&self) -> i32;

    /// Sets the total height of the content.
    fn set_total_width(&mut self, value: i32);

    /// Returns the visible height of the widget.
    fn viewport_width(&self) -> i32;

    /// Returns the current vertical scroll offset.
    fn scroll_offset(&self) -> i32;

    /// Sets the vertical scroll offset.
    fn set_scroll_offset(&mut self, offset: i32);

    /// Helper function to scroll by a certain amount (delta).
    /// This can be positive (to scroll down) or negative (to scroll up).
    fn scroll_by(&mut self, delta: i32) {
        let new_offset = (self.scroll_offset() + delta)
            .max(0)
            .min(self.total_width() - self.viewport_width());
        self.set_scroll_offset(new_offset);
    }

    /// Helper function to determine if the scrollbar is needed.
    fn needs_scrollbar(&self) -> bool;

    /// Get the maximum scroll offset for the thumb.
    fn max_thumb_offset(&self) -> i32 {
        self.viewport_width() - self.scrollbar_thumb_width()
    }

    /// Get the position of the scrollbar slider, accounting for the border.
    fn scrollbar_position(&self) -> f32 {
        if self.needs_scrollbar() {
            (self.scroll_offset() as f32 * self.max_thumb_offset() as f32)
                / (self.total_width() - self.viewport_width()) as f32
        } else {
            0.0
        }
    }

    /// Get the height of the scrollbar slider (thumb) as a proportion of the viewport height.
    fn scrollbar_thumb_width(&self) -> i32 {
        (self.viewport_width() as f32 * (self.viewport_width() as f32 / self.total_width() as f32))
            as i32
    }

    /// Adjust the scroll offset based on the mouse's delta movement on the thumb.
    fn adjust_scroll_from_thumb_delta(&mut self, delta_x: i32) {
        let thumb_height = self.scrollbar_thumb_width();
        let scale_factor = (self.total_width() - self.viewport_width()) as f32
            / (self.viewport_width() - thumb_height) as f32;

        let content_delta_x = (delta_x as f32 * scale_factor) as i32;

        self.scroll_by(content_delta_x);
    }

    /// Scroll content based on a click on the scrollbar track.
    fn scroll_from_track_click(&mut self, click_x: i32) {
        let thumb_top = self.scrollbar_position() as i32;
        let thumb_bottom = thumb_top + self.scrollbar_thumb_width();

        let new_offset;
        if click_x < thumb_top {
            // Page up
            new_offset = self.scroll_offset() - self.viewport_width();
        } else if click_x > thumb_bottom {
            // Page down
            new_offset = self.scroll_offset() + self.viewport_width();
        } else {
            return;
        }

        let clamped_offset = new_offset
            .max(0)
            .min(self.total_width() - self.viewport_width());
        self.set_scroll_offset(clamped_offset);
    }

    /// Adjust the scroll offset and total width based on a new zoom level.
    fn adjust_to_new_zoom_level(&mut self, new_zoom_level: f32, last_zoom_level: f32) {
        // Calculate the middle point of the currently visible content.
        let middle_point =
            (self.scroll_offset() + self.viewport_width() / 2) as f32 / last_zoom_level;

        // Calculate the new total width based on the new zoom level.
        let base_total_width = self.total_width() as f32 / last_zoom_level;
        let new_total_width = (base_total_width * new_zoom_level).round() as i32;

        // Calculate the new scroll offset so that the middle point remains visible.
        let new_scroll_offset =
            (middle_point * new_zoom_level - self.viewport_width() as f32 / 2.0).round() as i32;

        // Clamp the new scroll offset to ensure it is within bounds.
        let clamped_new_scroll_offset = new_scroll_offset
            .min(new_total_width - self.viewport_width())
            .max(0);

        // Set the new total width and scroll offset.
        self.set_total_width(new_total_width);
        self.set_scroll_offset(clamped_new_scroll_offset);
    }

    /// Scrolls to center the specified offset within the total width.
    fn scroll_to(&mut self, offset: i32) {
        let viewport_half_width = self.viewport_width() / 2;
        let new_scroll_offset = offset - viewport_half_width;

        // Clamp the new scroll offset to ensure it doesn't go out of bounds.
        let clamped_scroll_offset = new_scroll_offset
            .max(0)
            .min(self.total_width() - self.viewport_width());

        // Set the new scroll offset.
        self.set_scroll_offset(clamped_scroll_offset);
    }
}

impl TheHorizontalScrollbarTrait for TheHorizontalScrollbar {
    fn total_width(&self) -> i32 {
        self.total_width
    }

    fn set_total_width(&mut self, value: i32) {
        self.total_width = value;
    }

    fn viewport_width(&self) -> i32 {
        self.dim().width
    }

    fn scroll_offset(&self) -> i32 {
        self.scroll_offset
    }

    fn set_scroll_offset(&mut self, offset: i32) {
        self.scroll_offset = offset;
        self.is_dirty = true;
    }

    fn needs_scrollbar(&self) -> bool {
        self.total_width() > self.dim().width
    }
}
