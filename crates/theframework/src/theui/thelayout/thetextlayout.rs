use crate::prelude::*;

pub struct TheTextLayout {
    id: TheId,
    limiter: TheSizeLimiter,

    dim: TheDim,

    text: Vec<String>,
    text_rect: Vec<(usize, usize, usize, usize)>,

    widgets: Vec<Box<dyn TheWidget>>,

    list_buffer: TheRGBABuffer,

    vertical_scrollbar: Box<dyn TheWidget>,
    vertical_scrollbar_visible: bool,

    text_size: f32,
    text_margin: i32,
    fixed_text_width: Option<i32>,

    margin: Vec4<i32>,
    padding: i32,

    background: Option<TheThemeColors>,

    text_align: TheHorizontalAlign,
}

impl TheLayout for TheTextLayout {
    fn new(id: TheId) -> Self
    where
        Self: Sized,
    {
        Self {
            id,
            limiter: TheSizeLimiter::new(),

            dim: TheDim::zero(),

            text: vec![],
            text_rect: vec![],

            widgets: vec![],
            list_buffer: TheRGBABuffer::empty(),

            vertical_scrollbar: Box::new(TheVerticalScrollbar::new(TheId::named(
                "Vertical Scrollbar",
            ))),
            vertical_scrollbar_visible: false,

            text_size: 13.0,
            text_margin: 10,
            fixed_text_width: None,

            margin: Vec4::new(10, 10, 10, 10),
            padding: 10,

            background: Some(TextLayoutBackground),
            text_align: TheHorizontalAlign::Left,
        }
    }

    fn id(&self) -> &TheId {
        &self.id
    }

    fn set_margin(&mut self, margin: Vec4<i32>) {
        self.margin = margin;
    }

    fn set_padding(&mut self, padding: i32) {
        self.padding = padding;
    }

    fn set_background_color(&mut self, color: Option<TheThemeColors>) {
        self.background = color;
    }

    fn widgets(&mut self) -> &mut Vec<Box<dyn TheWidget>> {
        &mut self.widgets
    }

    fn get_widget_at_coord(&mut self, coord: Vec2<i32>) -> Option<&mut Box<dyn TheWidget>> {
        if !self.dim.contains(coord) {
            return None;
        }
        if self.vertical_scrollbar_visible && self.vertical_scrollbar.dim().contains(coord) {
            return Some(&mut self.vertical_scrollbar);
        }

        let mut scroll_offset = Vec2::new(0, 0);
        if let Some(scroll_bar) = self.vertical_scrollbar.as_vertical_scrollbar() {
            scroll_offset = Vec2::new(0, scroll_bar.scroll_offset());
        }

        let widgets = self.widgets();
        widgets
            .iter_mut()
            .find(|w| w.dim().contains(coord + scroll_offset))
    }

    fn get_widget(
        &mut self,
        name: Option<&String>,
        uuid: Option<&Uuid>,
    ) -> Option<&mut Box<dyn TheWidget>> {
        if self.vertical_scrollbar_visible && self.vertical_scrollbar.id().matches(name, uuid) {
            return Some(&mut self.vertical_scrollbar);
        }
        self.widgets.iter_mut().find(|w| w.id().matches(name, uuid))
    }

    fn needs_redraw(&mut self) -> bool {
        if self.vertical_scrollbar_visible && self.vertical_scrollbar.needs_redraw() {
            return true;
        }

        for i in 0..self.widgets.len() {
            if self.widgets[i].needs_redraw() {
                return true;
            }
        }

        if self.text.is_empty() {
            true
        } else {
            false
        }
    }

    fn dim(&self) -> &TheDim {
        &self.dim
    }

    fn dim_mut(&mut self) -> &mut TheDim {
        &mut self.dim
    }

    fn set_dim(&mut self, dim: TheDim, ctx: &mut TheContext) {
        if self.dim != dim || ctx.ui.relayout {
            self.dim = dim;

            let x = self.margin.x;
            let mut y = self.margin.y;

            // First pass calculate height to see if we need vertical scrollbar

            for w in &mut self.widgets.iter_mut() {
                w.calculate_size(ctx);
                let height = w.limiter().get_height(dim.height);
                y += height + self.padding;
            }
            let total_height = y - self.padding + self.margin.w;

            let width = dim.width;

            self.vertical_scrollbar
                .set_dim(TheDim::new(dim.x + width - 13, dim.y, 13, dim.height), ctx);
            self.vertical_scrollbar
                .dim_mut()
                .set_buffer_offset(self.dim.buffer_x + width - 13, self.dim.buffer_y);

            if let Some(scroll_bar) = self.vertical_scrollbar.as_vertical_scrollbar() {
                scroll_bar.set_total_height(total_height);
                self.vertical_scrollbar_visible = scroll_bar.needs_scrollbar();
            }

            y = self.margin.y;

            // Calculate text width
            let mut text_width = 0;

            for t in &mut self.text {
                let size = if !t.is_empty() {
                    ctx.draw.get_text_size(
                        t,
                        &TheFontSettings {
                            size: self.text_size,
                            ..Default::default()
                        },
                    )
                } else {
                    (0, 0)
                };
                if size.0 > text_width {
                    text_width = size.0;
                }
            }

            if let Some(fixed_text_width) = self.fixed_text_width {
                text_width = fixed_text_width as usize;
            }

            text_width += self.text_margin as usize + 5;

            // --

            let mut texts_rect: Vec<(usize, usize, usize, usize)> = vec![];
            let mut max_width = dim.width - text_width as i32 - self.margin.x - self.margin.z;

            if self.vertical_scrollbar_visible {
                max_width -= 13;
            }

            for (index, w) in &mut self.widgets.iter_mut().enumerate() {
                w.calculate_size(ctx);

                let text_is_empty = self.text[index].is_empty();

                let width = w.limiter().get_width(if text_is_empty {
                    max_width + text_width as i32
                } else {
                    max_width
                });
                let height = w.limiter().get_height(dim.height);

                // Limit to visible area
                // if y + height > dim.height {
                //     break;
                // }

                texts_rect.push((
                    (self.dim.buffer_x + x) as usize,
                    (self.dim.buffer_y + y) as usize,
                    text_width
                        - if text_width > self.text_margin as usize {
                            self.text_margin as usize
                        } else {
                            0
                        },
                    self.text_size as usize,
                ));

                if text_is_empty {
                    let offset = (max_width + text_width as i32 - width) / 2;
                    w.set_dim(
                        TheDim::new(dim.x + x + offset, dim.y + y, width, height),
                        ctx,
                    );
                    w.dim_mut()
                        .set_buffer_offset(self.dim.buffer_x + x + offset, self.dim.buffer_y + y);
                } else {
                    w.set_dim(
                        TheDim::new(dim.x + x + text_width as i32, dim.y + y, width, height),
                        ctx,
                    );
                    w.dim_mut().set_buffer_offset(
                        self.dim.buffer_x + x + text_width as i32,
                        self.dim.buffer_y + y,
                    );
                }

                y += height + self.padding;
            }

            let mut total_height = y - self.padding + self.margin.w;

            if total_height < dim.height {
                total_height = dim.height;
            }

            let mut width = dim.width;

            if self.vertical_scrollbar_visible {
                width -= 13;
            }

            self.list_buffer
                .set_dim(TheDim::new(0, 0, width, total_height));

            self.text_rect = texts_rect;
        }
    }

    fn limiter(&self) -> &TheSizeLimiter {
        &self.limiter
    }

    fn limiter_mut(&mut self) -> &mut TheSizeLimiter {
        &mut self.limiter
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

        let stride: usize = buffer.stride();
        if let Some(background) = self.background {
            ctx.draw.rect(
                buffer.pixels_mut(),
                &self.dim.to_buffer_utuple(),
                stride,
                style.theme().color(background),
            );

            // ctx.draw.rect_outline(
            //     buffer.pixels_mut(),
            //     &self.dim.to_buffer_utuple(),
            //     stride,
            //     style.theme().color(TextLayoutBorder),
            // );
        }

        let stride = self.list_buffer.stride();
        let utuple: (usize, usize, usize, usize) = self.list_buffer.dim().to_buffer_utuple();

        if let Some(background) = self.background {
            ctx.draw.rect(
                self.list_buffer.pixels_mut(),
                &utuple,
                stride,
                style.theme().color(background),
            );
        }

        if self.vertical_scrollbar_visible {
            self.vertical_scrollbar.draw(buffer, style, ctx);
        }

        for i in 0..self.text.len() {
            if self.text[i].is_empty() {
                continue;
            }
            let mut color = [240, 240, 240, 255];
            if self.widgets[i]
                .as_any()
                .downcast_ref::<TheSeparator>()
                .is_some()
            {
                color = [160, 160, 160, 255];
            }

            ctx.draw.text_rect_blend(
                self.list_buffer.pixels_mut(),
                &self.text_rect[i],
                stride,
                &self.text[i],
                TheFontSettings {
                    size: self.text_size,
                    ..Default::default()
                },
                &color,
                self.text_align.clone(),
                TheVerticalAlign::Top,
            );
        }

        for w in &mut self.widgets {
            w.draw(&mut self.list_buffer, style, ctx);
        }

        if self.vertical_scrollbar_visible {
            if let Some(scroll_bar) = self.vertical_scrollbar.as_vertical_scrollbar() {
                let offset = scroll_bar.scroll_offset();
                let range = offset..offset + self.dim.height;
                buffer.copy_vertical_range_into(
                    self.dim.buffer_x,
                    self.dim.buffer_y,
                    &self.list_buffer,
                    range,
                );
            }
        } else if let Some(scroll_bar) = self.vertical_scrollbar.as_vertical_scrollbar() {
            let range = 0..scroll_bar.total_height();
            buffer.copy_vertical_range_into(
                self.dim.buffer_x,
                self.dim.buffer_y,
                &self.list_buffer,
                range,
            );
        }

        if self.background.is_some() {
            let stride: usize = buffer.stride();
            ctx.draw.rect_outline(
                buffer.pixels_mut(),
                &self.dim.to_buffer_utuple(),
                stride,
                style.theme().color(TextLayoutBorder),
            );
        }
    }

    fn as_text_layout(&mut self) -> Option<&mut dyn TheTextLayoutTrait> {
        Some(self)
    }
}

/// TheTextLayout specific functions.
pub trait TheTextLayoutTrait: TheLayout {
    /// Clear the text and widget pairs.
    fn clear(&mut self);
    /// Add a text / widget pair.
    fn add_pair(&mut self, text: String, widget: Box<dyn TheWidget>);
    /// Set the fixed text width.
    fn set_fixed_text_width(&mut self, text_width: i32);
    /// Set the text size to use for the left handed text.
    fn set_text_size(&mut self, text_size: f32);
    /// Set the text margin between the text and the widget.
    fn set_text_margin(&mut self, text_margin: i32);
    /// The horizontal text alignment
    fn set_text_align(&mut self, align: TheHorizontalAlign);
}

impl TheTextLayoutTrait for TheTextLayout {
    fn clear(&mut self) {
        self.text.clear();
        self.widgets.clear();
    }
    fn add_pair(&mut self, text: String, widget: Box<dyn TheWidget>) {
        self.text.push(text);
        self.widgets.push(widget);
    }
    fn set_fixed_text_width(&mut self, text_width: i32) {
        self.fixed_text_width = Some(text_width);
    }
    fn set_text_size(&mut self, text_size: f32) {
        self.text_size = text_size;
    }
    fn set_text_margin(&mut self, text_margin: i32) {
        self.text_margin = text_margin;
    }
    fn set_text_align(&mut self, align: TheHorizontalAlign) {
        self.text_align = align;
    }
}
