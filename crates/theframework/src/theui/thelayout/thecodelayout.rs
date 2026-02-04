use crate::prelude::*;

pub struct TheCodeLayout {
    id: TheId,
    limiter: TheSizeLimiter,

    dim: TheDim,

    widgets: Vec<Box<dyn TheWidget>>,

    code_view: Box<dyn TheWidget>,

    vertical_scrollbar: Box<dyn TheWidget>,
    vertical_scrollbar_visible: bool,

    horizontal_scrollbar: Box<dyn TheWidget>,
    horizontal_scrollbar_visible: bool,

    margin: Vec4<i32>,

    background: Option<TheThemeColors>,
}

impl TheLayout for TheCodeLayout {
    fn new(id: TheId) -> Self
    where
        Self: Sized,
    {
        let name = id.name.clone();

        let mut code_view = TheCodeView::new(TheId::named((name.clone() + " View").as_str()));
        let vertical_scrollbar = Box::new(TheVerticalScrollbar::new(TheId::named(
            (name.clone() + " Vertical Scrollbar").as_str(),
        )));

        let horizontal_scrollbar = Box::new(TheHorizontalScrollbar::new(TheId::named(
            (name + " Horizontal Scrollbar").as_str(),
        )));

        code_view.set_scrollbar_ids(
            horizontal_scrollbar.id().clone(),
            vertical_scrollbar.id().clone(),
        );

        Self {
            id,
            limiter: TheSizeLimiter::new(),

            dim: TheDim::zero(),

            widgets: vec![],

            code_view: Box::new(code_view),

            vertical_scrollbar,
            vertical_scrollbar_visible: false,

            horizontal_scrollbar,
            horizontal_scrollbar_visible: false,

            margin: Vec4::new(0, 0, 0, 0),

            background: Some(TextLayoutBackground),
        }
    }

    fn id(&self) -> &TheId {
        &self.id
    }

    fn set_margin(&mut self, margin: Vec4<i32>) {
        self.margin = margin;
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

        if self.horizontal_scrollbar_visible && self.horizontal_scrollbar.dim().contains(coord) {
            return Some(&mut self.horizontal_scrollbar);
        }

        let mut scroll_offset = Vec2::new(0, 0);
        if let Some(scroll_bar) = self.vertical_scrollbar.as_vertical_scrollbar() {
            scroll_offset = Vec2::new(0, scroll_bar.scroll_offset());
        }

        if self.code_view.dim().contains(coord) {
            return Some(&mut self.code_view);
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

        if self.horizontal_scrollbar_visible && self.horizontal_scrollbar.id().matches(name, uuid) {
            return Some(&mut self.horizontal_scrollbar);
        }

        if self.code_view.id().matches(name, uuid) {
            return Some(&mut self.code_view);
        }

        self.widgets.iter_mut().find(|w| w.id().matches(name, uuid))
    }

    fn needs_redraw(&mut self) -> bool {
        if self.vertical_scrollbar_visible && self.vertical_scrollbar.needs_redraw() {
            return true;
        }

        if self.horizontal_scrollbar_visible && self.horizontal_scrollbar.needs_redraw() {
            return true;
        }

        for i in 0..self.widgets.len() {
            if self.widgets[i].needs_redraw() {
                return true;
            }
        }
        false
    }

    fn relayout(&mut self, ctx: &mut TheContext) {
        let dim = self.dim;
        self.dim = TheDim::zero();
        self.set_dim(dim, ctx);
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

            let mut width: i32 = dim.width;
            let mut height: i32 = dim.height;

            let mut buffer_dim = TheDim::zero();

            let mut zoom: f32 = 1.0;

            if let Some(code_view) = self.code_view.as_code_view() {
                _ = code_view.adjust_buffer_to_grid();
                buffer_dim = *code_view.buffer().dim();
                zoom = code_view.zoom();
            }

            // Vertical

            self.vertical_scrollbar.set_dim(
                TheDim::new(dim.x + width - 13, dim.y, 13, dim.height - 13),
                ctx,
            );
            self.vertical_scrollbar
                .dim_mut()
                .set_buffer_offset(self.dim.buffer_x + width - 13, self.dim.buffer_y);

            if let Some(scroll_bar) = self.vertical_scrollbar.as_vertical_scrollbar() {
                scroll_bar.set_total_height((buffer_dim.height as f32 * zoom) as i32);
                self.vertical_scrollbar_visible = scroll_bar.needs_scrollbar();
            }

            // Horizontal

            self.horizontal_scrollbar
                .set_dim(TheDim::new(dim.x, dim.y + height - 13, width - 13, 13), ctx);
            self.horizontal_scrollbar
                .dim_mut()
                .set_buffer_offset(self.dim.buffer_x, self.dim.buffer_y + height - 13);

            if let Some(scroll_bar) = self.horizontal_scrollbar.as_horizontal_scrollbar() {
                scroll_bar.set_total_width((buffer_dim.width as f32 * zoom) as i32);
                self.horizontal_scrollbar_visible = scroll_bar.needs_scrollbar();
            }

            if self.vertical_scrollbar_visible || self.horizontal_scrollbar_visible {
                width -= 13;
                height -= 13;
            }

            self.code_view
                .set_dim(TheDim::new(dim.x, dim.y, width, height), ctx);
            self.code_view
                .dim_mut()
                .set_buffer_offset(self.dim.buffer_x, self.dim.buffer_y);
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

        //

        let mut scroll_offset = Vec2::new(0, 0);

        if let Some(scroll_bar) = self.vertical_scrollbar.as_vertical_scrollbar() {
            scroll_offset.y = scroll_bar.scroll_offset();
        }

        if let Some(scroll_bar) = self.horizontal_scrollbar.as_horizontal_scrollbar() {
            scroll_offset.x = scroll_bar.scroll_offset();
        }

        if let Some(rgba_view) = self.code_view.as_code_view() {
            rgba_view.set_scroll_offset(scroll_offset);
        }

        self.code_view.draw(buffer, style, ctx);

        if self.vertical_scrollbar_visible || self.horizontal_scrollbar_visible {
            self.vertical_scrollbar.draw(buffer, style, ctx);
            self.horizontal_scrollbar.draw(buffer, style, ctx);

            let stride: usize = buffer.stride();

            let utuple = (
                (self.dim().buffer_x + self.dim.width - 13) as usize,
                (self.dim.buffer_y + self.dim.height - 13) as usize,
                13,
                13,
            );

            ctx.draw.rect(
                buffer.pixels_mut(),
                &utuple,
                stride,
                style.theme().color(ScrollbarBackground),
            );

            let utuple = (
                (self.dim().buffer_x) as usize,
                (self.dim.buffer_y + self.dim.height - 13) as usize,
                self.dim.width as usize,
                1,
            );

            ctx.draw.rect(
                buffer.pixels_mut(),
                &utuple,
                stride,
                style.theme().color(ScrollbarSeparator),
            );

            let utuple = (
                (self.dim().buffer_x + self.dim.width - 13) as usize,
                (self.dim.buffer_y) as usize,
                1,
                self.dim.height as usize,
            );

            ctx.draw.rect(
                buffer.pixels_mut(),
                &utuple,
                stride,
                style.theme().color(ScrollbarSeparator),
            );
        }
    }

    /// Convert to the code layout trait
    fn as_code_layout(&mut self) -> Option<&mut dyn TheCodeLayoutTrait> {
        Some(self)
    }
}

/// TheRGBALayout specific functions.
pub trait TheCodeLayoutTrait {
    /// Set the buffer to be displayed.
    fn set_buffer(&mut self, buffer: TheRGBABuffer);
    /// Get the current scroll offset for the scrollbars.
    fn scroll_offset(&mut self) -> Vec2<i32>;
    /// Set the scroll offset for the scrollbars.
    fn set_scroll_offset(&mut self, offset: Vec2<i32>);
    /// Returns a mutable reference to the underlying TheRGBAView.
    fn code_view_mut(&mut self) -> &mut Box<dyn TheWidget>;
}

impl TheCodeLayoutTrait for TheCodeLayout {
    fn set_buffer(&mut self, buffer: TheRGBABuffer) {
        if let Some(rgba) = self.code_view.as_rgba_view() {
            rgba.set_buffer(buffer);
        }
    }
    fn scroll_offset(&mut self) -> Vec2<i32> {
        let mut offset = Vec2::zero();
        if let Some(scroll_bar) = self.vertical_scrollbar.as_vertical_scrollbar() {
            offset.y = scroll_bar.scroll_offset();
        }
        if let Some(scroll_bar) = self.horizontal_scrollbar.as_horizontal_scrollbar() {
            offset.x = scroll_bar.scroll_offset();
        }
        offset
    }
    fn set_scroll_offset(&mut self, offset: Vec2<i32>) {
        if let Some(scroll_bar) = self.vertical_scrollbar.as_vertical_scrollbar() {
            scroll_bar.set_scroll_offset(offset.y);
        }
        if let Some(scroll_bar) = self.horizontal_scrollbar.as_horizontal_scrollbar() {
            scroll_bar.set_scroll_offset(offset.x);
        }
    }
    fn code_view_mut(&mut self) -> &mut Box<dyn TheWidget> {
        &mut self.code_view
    }
}
