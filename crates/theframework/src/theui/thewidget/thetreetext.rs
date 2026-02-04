use crate::prelude::*;

pub struct TheTreeText {
    id: TheId,
    limiter: TheSizeLimiter,

    state: TheWidgetState,

    dim: TheDim,
    is_dirty: bool,

    layout_id: TheId,
    scroll_offset: i32,

    text: String,
    font_size: f32,
    line_height: i32,
    padding_left: i32,
    padding_top: i32,
    padding_bottom: i32,

    lines: Vec<String>,

    context_menu: Option<TheContextMenu>,
}

impl TheWidget for TheTreeText {
    fn new(id: TheId) -> Self
    where
        Self: Sized,
    {
        let mut limiter = TheSizeLimiter::new();
        limiter.set_max_height(22);

        Self {
            id,
            limiter,

            state: TheWidgetState::None,

            dim: TheDim::zero(),
            is_dirty: true,

            layout_id: TheId::empty(),
            scroll_offset: 0,

            text: String::new(),
            font_size: 13.0,
            line_height: 18,
            padding_left: 9,
            padding_top: 5,
            padding_bottom: 5,

            lines: vec![],

            context_menu: None,
        }
    }

    fn id(&self) -> &TheId {
        &self.id
    }

    fn calculate_size(&mut self, _ctx: &mut TheContext) {
        // Re-wrap text during layout using current dimension width
        // The widget should already have a dimension from previous layout or from set_dim
        if self.dim.width > 0 {
            self.wrap_text();
        } else if !self.text.is_empty() {
            // If no dimension yet, estimate a single line
            self.lines = vec![self.text.clone()];
            self.update_height();
        }
    }

    fn set_context_menu(&mut self, menu: Option<TheContextMenu>) {
        self.context_menu = menu;
    }

    fn on_event(&mut self, event: &TheEvent, ctx: &mut TheContext) -> bool {
        let mut redraw = false;

        match event {
            TheEvent::Context(coord) => {
                if let Some(context_menu) = &self.context_menu {
                    ctx.ui.send(TheEvent::ShowContextMenu(
                        self.id().clone(),
                        *coord,
                        context_menu.clone(),
                    ));
                    ctx.ui.set_focus(self.id());
                    redraw = true;
                    self.is_dirty = true;
                }
            }
            TheEvent::MouseDown(_coord) => {
                if self.state != TheWidgetState::Selected || !self.id().equals(&ctx.ui.focus) {
                    self.is_dirty = true;
                    self.state = TheWidgetState::Selected;
                    ctx.ui.send_widget_state_changed(self.id(), self.state);
                    ctx.ui.send(TheEvent::NewListItemSelected(
                        self.id().clone(),
                        self.layout_id.clone(),
                    ));
                    redraw = true;
                }
                ctx.ui.set_focus(self.id());
            }
            TheEvent::Hover(_coord) => {
                if self.state != TheWidgetState::Selected && !self.id().equals(&ctx.ui.hover) {
                    self.is_dirty = true;
                    ctx.ui.set_hover(self.id());
                    redraw = true;
                }
            }
            TheEvent::MouseWheel(delta) => {
                ctx.ui
                    .send(TheEvent::ScrollLayout(self.layout_id.clone(), *delta));
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
        // Always update position and width
        let width_changed = self.dim.width != dim.width;
        self.dim.x = dim.x;
        self.dim.y = dim.y;
        self.dim.width = dim.width;
        self.dim.buffer_x = dim.buffer_x;
        self.dim.buffer_y = dim.buffer_y;
        self.is_dirty = true;

        // If width changed and we have text, rewrap to get correct height
        if dim.width > 0 && !self.text.is_empty() && width_changed {
            self.wrap_text();

            // Override the height with our calculated height (don't use the one passed in)
            let new_height = self.limiter.get_max_height() + 2; // +2 for padding
            self.dim.height = new_height;
        } else {
            // Use the height from dim if we're not wrapping
            self.dim.height = dim.height;
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
        TheValue::Text(self.text.clone())
    }

    fn set_value(&mut self, value: TheValue) {
        if let TheValue::Text(text) = value {
            self.text = text;
            self.wrap_text();
            self.is_dirty = true;
        }
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

        // Fundamental safety check: ensure we don't draw outside buffer bounds
        let buffer_width = buffer.dim().width as i32;
        let buffer_height = buffer.dim().height as i32;
        let item_buffer_x = self.dim().buffer_x;
        let item_buffer_y = self.dim().buffer_y;

        if item_buffer_x < 0
            || item_buffer_y < 0
            || item_buffer_x + self.dim().width > buffer_width
            || item_buffer_y + self.dim().height > buffer_height
        {
            return;
        }

        let color = if self.state == TheWidgetState::Selected {
            if !self.id().equals(&ctx.ui.focus) {
                *style.theme().color(ListItemSelectedNoFocus)
            } else {
                *style.theme().color(ListItemSelected)
            }
        } else {
            *style.theme().color(ListItemNormal)
        };

        let hover_color =
            if self.state != TheWidgetState::Selected && self.id().equals(&ctx.ui.hover) {
                *style.theme().color(ListItemHover)
            } else {
                color
            };

        let stride = buffer.stride();
        let shrinker = TheDimShrinker::zero();

        // Draw background outline
        let mut adjusted_utuple = self.dim.to_buffer_shrunk_utuple(&shrinker);
        adjusted_utuple.1 += 1; // Draw 1px lower
        adjusted_utuple.3 = adjusted_utuple.3.saturating_sub(2); // Reduce height by 2px
        let buffer_width = buffer.dim().width as usize;
        let buffer_height = buffer.dim().height as usize;

        if adjusted_utuple.0 < buffer_width
            && adjusted_utuple.1 < buffer_height
            && adjusted_utuple.0 + adjusted_utuple.2 <= buffer_width
            && adjusted_utuple.1 + adjusted_utuple.3 <= buffer_height
        {
            ctx.draw.rect_outline_border_open(
                buffer.pixels_mut(),
                &adjusted_utuple,
                stride,
                &hover_color,
                1,
            );
        }

        let mut shrinker = TheDimShrinker::zero();
        shrinker.shrink(1);

        // Draw background fill
        let mut adjusted_utuple = self.dim.to_buffer_shrunk_utuple(&shrinker);
        adjusted_utuple.1 += 1; // Draw 1px lower
        adjusted_utuple.3 = adjusted_utuple.3.saturating_sub(2); // Reduce height by 2px
        let buffer_width = buffer.dim().width as usize;
        let buffer_height = buffer.dim().height as usize;

        if adjusted_utuple.0 < buffer_width
            && adjusted_utuple.1 < buffer_height
            && adjusted_utuple.0 + adjusted_utuple.2 <= buffer_width
            && adjusted_utuple.1 + adjusted_utuple.3 <= buffer_height
        {
            ctx.draw
                .rect(buffer.pixels_mut(), &adjusted_utuple, stride, &hover_color);
        }

        // Draw text lines
        let text_color = *style.theme().color(ListItemText);

        // Calculate vertical centering for single line
        let total_text_height = self.lines.len() as i32 * self.line_height;
        let available_height = adjusted_utuple.3 as i32 - 2; // Subtract the 2px adjustment
        let vertical_offset = if self.lines.len() == 1 {
            // Center single line vertically
            ((available_height - total_text_height) / 2).max(0)
        } else {
            // Multiple lines use padding_top
            self.padding_top
        };

        let mut y_offset = vertical_offset as usize;

        for line in &self.lines {
            let text_rect = (
                adjusted_utuple.0 + self.padding_left as usize,
                adjusted_utuple.1 + y_offset,
                adjusted_utuple
                    .2
                    .saturating_sub(self.padding_left as usize * 2),
                self.line_height as usize,
            );

            // Safety check: ensure text is within buffer bounds before drawing
            if text_rect.0 < buffer_width
                && text_rect.1 < buffer_height
                && text_rect.0 + text_rect.2 <= buffer_width
                && text_rect.1 + text_rect.3 <= buffer_height
            {
                ctx.draw.text_rect_blend(
                    buffer.pixels_mut(),
                    &text_rect,
                    stride,
                    line,
                    TheFontSettings {
                        size: self.font_size,
                        ..Default::default()
                    },
                    &text_color,
                    TheHorizontalAlign::Left,
                    TheVerticalAlign::Center,
                );
            }

            y_offset += self.line_height as usize;
        }

        self.is_dirty = false;
    }

    fn as_tree_text(&mut self) -> Option<&mut dyn TheTreeTextTrait> {
        Some(self)
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl TheTreeText {
    /// Wrap text into multiple lines based on available width
    fn wrap_text(&mut self) {
        self.lines.clear();

        if self.text.is_empty() || self.dim.width <= self.padding_left * 2 {
            self.update_height();
            return;
        }

        let available_width = self.dim.width - (self.padding_left * 2);

        // Split text into words
        let words: Vec<&str> = self.text.split_whitespace().collect();

        if words.is_empty() {
            self.update_height();
            return;
        }

        let mut current_line = String::new();

        for word in words {
            let test_line = if current_line.is_empty() {
                word.to_string()
            } else {
                format!("{} {}", current_line, word)
            };

            // Approximate text width (rough estimation: char_width * font_size * 0.6)
            let estimated_width = (test_line.len() as f32 * self.font_size * 0.6) as i32;

            if estimated_width <= available_width {
                current_line = test_line;
            } else {
                if !current_line.is_empty() {
                    self.lines.push(current_line.clone());
                }
                current_line = word.to_string();
            }
        }

        // Push the last line
        if !current_line.is_empty() {
            self.lines.push(current_line);
        }

        self.update_height();
    }

    /// Calculate and update the height based on the number of lines
    fn update_height(&mut self) {
        if self.lines.is_empty() {
            self.limiter.set_max_height(22);
            return;
        }

        let height = self.padding_top
            + self.padding_bottom
            + (self.lines.len() as i32 * self.line_height)
            + 4;
        self.limiter.set_max_height(height);
    }
}

pub trait TheTreeTextTrait {
    fn set_associated_layout(&mut self, id: TheId);
    fn set_scroll_offset(&mut self, offset: i32);
    fn set_text(&mut self, text: String);
    fn set_font_size(&mut self, size: f32);
    fn set_line_height(&mut self, height: i32);
    fn set_padding(&mut self, left: i32, top: i32, bottom: i32);
}

impl TheTreeTextTrait for TheTreeText {
    fn set_associated_layout(&mut self, layout_id: TheId) {
        self.layout_id = layout_id;
    }

    fn set_scroll_offset(&mut self, offset: i32) {
        self.scroll_offset = offset;
    }

    fn set_text(&mut self, text: String) {
        self.text = text;
        // Only wrap text if we have a valid dimension, otherwise it will be wrapped
        // during calculate_size when the layout is computed
        if self.dim.width > 0 {
            self.wrap_text();
        }
        // Don't set any height here - let calculate_size handle it during layout
        self.is_dirty = true;
    }

    fn set_font_size(&mut self, size: f32) {
        self.font_size = size;
        self.wrap_text();
        self.is_dirty = true;
    }

    fn set_line_height(&mut self, height: i32) {
        self.line_height = height;
        self.update_height();
        self.is_dirty = true;
    }

    fn set_padding(&mut self, left: i32, top: i32, bottom: i32) {
        self.padding_left = left;
        self.padding_top = top;
        self.padding_bottom = bottom;
        self.update_height();
        self.is_dirty = true;
    }
}
