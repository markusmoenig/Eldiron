use crate::prelude::*;

pub struct TheGroupButton {
    id: TheId,
    limiter: TheSizeLimiter,
    state: TheWidgetState,

    text: Vec<String>,
    status_text: Vec<Option<String>>,
    icon: Vec<Option<String>>,

    hover_index: Option<usize>,
    selected_index: Option<usize>,

    icon_size: Vec2<i32>,
    item_width: usize,

    dim: TheDim,

    is_disabled: bool,
    is_dirty: bool,
}

impl TheWidget for TheGroupButton {
    fn new(id: TheId) -> Self
    where
        Self: Sized,
    {
        let mut limiter = TheSizeLimiter::new();
        limiter.set_max_size(Vec2::new(60, 20));
        Self {
            id,
            limiter,
            state: TheWidgetState::None,

            text: vec![],
            status_text: vec![],

            icon: vec![],

            hover_index: None,
            selected_index: Some(0),

            icon_size: Vec2::new(18, 18),
            item_width: 60,

            dim: TheDim::zero(),

            is_disabled: false,
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
            TheEvent::MouseDown(coord) => {
                if self.state != TheWidgetState::Selected {
                    self.state = TheWidgetState::Selected;
                    ctx.ui.send_widget_state_changed(self.id(), self.state);
                }
                let index = coord.x as usize / (self.item_width + 1);
                ctx.ui.send(TheEvent::IndexChanged(self.id.clone(), index));
                self.selected_index = Some(index);
                self.is_dirty = true;
                redraw = true;
            }
            TheEvent::Hover(coord) => {
                if !self.id().equals(&ctx.ui.hover) {
                    self.is_dirty = true;
                    ctx.ui.set_hover(self.id());
                    redraw = true;
                }
                let index = coord.x as usize / (self.item_width + 1);
                if let Some(text) = self.status_text[index].clone() {
                    ctx.ui
                        .send(TheEvent::SetStatusText(self.id.clone(), text.clone()));
                }
                if Some(index) != self.hover_index {
                    if Some(index) != self.selected_index {
                        self.hover_index = Some(index);
                    } else {
                        self.hover_index = None;
                    }
                    redraw = true;
                    self.is_dirty = true;
                }
            }
            TheEvent::LostHover(_id) => {
                self.hover_index = None;
                redraw = true;
                self.is_dirty = true;
            }
            _ => {}
        }
        redraw
    }

    fn calculate_size(&mut self, _ctx: &mut TheContext) {
        let mut width = self.text.len() * self.item_width;
        if !self.text.is_empty() {
            width += self.text.len() - 1;
        }
        self.limiter.set_max_width(width as i32);
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

    fn supports_hover(&mut self) -> bool {
        true
    }

    fn value(&self) -> TheValue {
        TheValue::Int(self.selected_index.unwrap_or(0) as i32)
    }

    #[allow(clippy::single_match)]
    fn set_value(&mut self, value: TheValue) {
        match value {
            TheValue::Int(value) => {
                self.selected_index = Some(value as usize);
                self.is_dirty = true;
            }
            _ => {}
        }
    }

    fn draw(
        &mut self,
        buffer: &mut TheRGBABuffer,
        style: &mut Box<dyn TheStyle>,
        ctx: &mut TheContext,
    ) {
        let stride: usize = buffer.stride();

        if !self.dim().is_valid() {
            return;
        }

        let ut = self.dim.to_buffer_utuple();

        //style.draw_widget_border(buffer, self, &mut shrinker, ctx);

        let total = self.text.len() as i32;

        let mut x = 0;

        for (index, text) in self.text.iter().enumerate() {
            let border;
            let bg;

            if self.selected_index == Some(index) {
                border = *style.theme().color(GroupButtonSelectedBorder);
                bg = *style.theme().color(GroupButtonSelectedBackground);
            } else if self.hover_index == Some(index) {
                border = *style.theme().color(GroupButtonHoverBorder);
                bg = *style.theme().color(GroupButtonHoverBackground);
            } else {
                border = *style.theme().color(GroupButtonNormalBorder);
                bg = *style.theme().color(GroupButtonNormalBackground);
            }

            if index == 0 {
                // First

                ctx.draw.rect_outline_border(
                    buffer.pixels_mut(),
                    &(ut.0 + x, ut.1, self.item_width, 20),
                    stride,
                    &border,
                    1,
                );

                ctx.draw.rect(
                    buffer.pixels_mut(),
                    &(ut.0 + x + self.item_width - 1, ut.1, 1, 20),
                    stride,
                    &border,
                );

                ctx.draw.rect(
                    buffer.pixels_mut(),
                    &(ut.0 + x + 1, ut.1 + 1, self.item_width - 1, 18),
                    stride,
                    &bg,
                );
            } else if index == total as usize - 1 {
                // Last

                ctx.draw.rect_outline_border(
                    buffer.pixels_mut(),
                    &(ut.0 + x, ut.1, self.item_width, 20),
                    stride,
                    &border,
                    1,
                );

                ctx.draw.rect(
                    buffer.pixels_mut(),
                    &(ut.0 + x, ut.1, self.item_width - 2, 20),
                    stride,
                    &border,
                );

                ctx.draw.rect(
                    buffer.pixels_mut(),
                    &(ut.0 + x, ut.1 + 1, self.item_width - 1, 18),
                    stride,
                    &bg,
                );
            } else {
                ctx.draw.rect_outline(
                    buffer.pixels_mut(),
                    &(ut.0 + x, ut.1, self.item_width, 20),
                    stride,
                    &border,
                );

                ctx.draw.rect(
                    buffer.pixels_mut(),
                    &(ut.0 + x, ut.1 + 1, self.item_width, 18),
                    stride,
                    &bg,
                );
            }

            let mut has_icon = false;
            let mut offset = 0;

            if let Some(icon_name) = self.icon[index].clone() {
                if let Some(icon) = ctx.ui.icon(&icon_name) {
                    let r = (
                        ut.0 + x + 5,
                        ((ut.1 + (20 - self.icon_size.y as usize) / 2) as i32) as usize,
                        self.icon_size.x as usize,
                        self.icon_size.y as usize,
                    );
                    ctx.draw.blend_scale_chunk(
                        buffer.pixels_mut(),
                        &r,
                        stride,
                        icon.pixels(),
                        &(icon.dim().width as usize, icon.dim().height as usize),
                    );
                    // ctx.draw
                    // .blend_slice(buffer.pixels_mut(), icon.pixels(), &r, stride);
                    has_icon = true;
                    offset = self.icon_size.x as usize + 5 + 2;
                }
            }

            ctx.draw.text_rect_blend(
                buffer.pixels_mut(),
                &(ut.0 + x + offset + 1, ut.1 + 1, self.item_width - 2, 18),
                stride,
                text,
                TheFontSettings {
                    size: 12.5,
                    ..Default::default()
                },
                &WHITE,
                if has_icon {
                    TheHorizontalAlign::Left
                } else {
                    TheHorizontalAlign::Center
                },
                TheVerticalAlign::Center,
            );

            x += self.item_width;
            if (index as i32) < total {
                x += 1;
            }
        }

        self.is_dirty = false;
    }

    fn as_group_button(&mut self) -> Option<&mut dyn TheGroupButtonTrait> {
        Some(self)
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub trait TheGroupButtonTrait: TheWidget {
    /// Add a new text based item.
    fn add_text(&mut self, text: String);
    /// Add a new text based item with an associated status text.
    fn add_text_status(&mut self, text: String, status: String);
    /// Add a new text based item with an associated status text and an icon.
    fn add_text_status_icon(&mut self, text: String, status: String, icon: String);
    /// Set the width of each itme.
    fn set_item_width(&mut self, width: usize);
    /// Set the index.
    fn set_index(&mut self, index: i32);
    /// Get the selected index.
    fn index(&self) -> i32;
}

impl TheGroupButtonTrait for TheGroupButton {
    fn add_text(&mut self, text: String) {
        self.text.push(text);
        self.status_text.push(None);
        self.icon.push(None);
    }
    fn add_text_status(&mut self, text: String, status: String) {
        self.text.push(text);
        self.status_text.push(Some(status));
        self.icon.push(None);
    }
    fn add_text_status_icon(&mut self, text: String, status: String, icon: String) {
        self.text.push(text);
        self.status_text.push(Some(status));
        self.icon.push(Some(icon));
    }
    fn set_item_width(&mut self, width: usize) {
        self.item_width = width;
    }
    fn set_index(&mut self, index: i32) {
        self.selected_index = Some(index as usize);
        self.is_dirty = true;
    }
    fn index(&self) -> i32 {
        self.selected_index.unwrap_or(0) as i32
    }
}
