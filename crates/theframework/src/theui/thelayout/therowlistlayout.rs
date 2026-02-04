use crate::prelude::*;

pub struct TheRowListLayout {
    id: TheId,
    limiter: TheSizeLimiter,

    dim: TheDim,

    widgets: Vec<Box<dyn TheWidget>>,

    list_buffer: TheRGBABuffer,

    horizontal_scrollbar: Box<dyn TheWidget>,
    horizontal_scrollbar_visible: bool,

    margin: Vec4<i32>,

    background: Option<TheThemeColors>,
    item_size: i32,

    is_dirty: bool,
}

impl TheLayout for TheRowListLayout {
    fn new(id: TheId) -> Self
    where
        Self: Sized,
    {
        Self {
            id,
            limiter: TheSizeLimiter::new(),

            dim: TheDim::zero(),

            widgets: vec![],
            list_buffer: TheRGBABuffer::empty(),

            horizontal_scrollbar: Box::new(TheHorizontalScrollbar::new(TheId::named(
                "Horizontal Scrollbar",
            ))),
            horizontal_scrollbar_visible: false,

            margin: Vec4::new(3, 3, 3, 3),

            background: Some(TextLayoutBackground),
            item_size: 115,

            is_dirty: false,
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

        if self.horizontal_scrollbar_visible && self.horizontal_scrollbar.dim().contains(coord) {
            return Some(&mut self.horizontal_scrollbar);
        }

        let mut scroll_offset = Vec2::new(0, 0);
        if let Some(scroll_bar) = self.horizontal_scrollbar.as_horizontal_scrollbar() {
            scroll_offset = Vec2::new(scroll_bar.scroll_offset(), 0);
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
        if self.horizontal_scrollbar_visible && self.horizontal_scrollbar.id().matches(name, uuid) {
            return Some(&mut self.horizontal_scrollbar);
        }

        self.widgets.iter_mut().find(|w| w.id().matches(name, uuid))
    }

    fn needs_redraw(&mut self) -> bool {
        if self.horizontal_scrollbar_visible && self.horizontal_scrollbar.needs_redraw() {
            return true;
        }

        for i in 0..self.widgets.len() {
            if self.widgets[i].needs_redraw() {
                return true;
            }
        }

        self.is_dirty
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

            let mut x = 2;
            let y = 2;
            let mut height = dim.height;

            self.item_size = dim.height - 13 - 20 - 2;

            let items = self.widgets.len() as i32;
            let mut total_width = 2 + items * self.item_size + 2;
            if items > 0 {
                total_width += (items - 1) * 3;
            }

            if total_width < dim.width {
                total_width = dim.width;
            }

            self.horizontal_scrollbar
                .set_dim(TheDim::new(dim.x, dim.y + height - 13, dim.width, 13), ctx);
            self.horizontal_scrollbar
                .dim_mut()
                .set_buffer_offset(self.dim.buffer_x, self.dim.buffer_y + height - 13);

            if let Some(scroll_bar) = self.horizontal_scrollbar.as_horizontal_scrollbar() {
                scroll_bar.set_total_width(total_width);
                self.horizontal_scrollbar_visible = true; //scroll_bar.needs_scrollbar();
            }

            if self.horizontal_scrollbar_visible {
                height -= 13;
            }

            self.list_buffer
                .set_dim(TheDim::new(0, 0, total_width, height));

            for index in 0..items {
                let i = index as usize;

                self.widgets[i].set_dim(
                    TheDim::new(dim.x + x, dim.y + y, self.item_size, height - 4),
                    ctx,
                );
                self.widgets[i].dim_mut().set_buffer_offset(x, y);

                x += self.item_size + 3;
            }
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

        let stride = self.list_buffer.stride();
        let utuple: (usize, usize, usize, usize) = self.list_buffer.dim().to_buffer_utuple();

        ctx.draw.rect(
            self.list_buffer.pixels_mut(),
            &utuple,
            stride,
            style.theme().color(ListLayoutBackground),
        );

        ctx.draw.rect_outline(
            self.list_buffer.pixels_mut(),
            &utuple,
            stride,
            style.theme().color(ListLayoutBorder),
        );

        if self.horizontal_scrollbar_visible {
            self.horizontal_scrollbar.draw(buffer, style, ctx);
        }

        let mut offset = 0;

        if self.horizontal_scrollbar_visible {
            if let Some(scroll_bar) = self.horizontal_scrollbar.as_horizontal_scrollbar() {
                offset = scroll_bar.scroll_offset();
            }
        }

        let items = self.widgets.len();

        for i in 0..items {
            self.widgets[i].draw(&mut self.list_buffer, style, ctx);
            if let Some(item) = self.widgets[i].as_list_item() {
                item.set_scroll_offset(offset);
            }
        }

        if self.horizontal_scrollbar_visible {
            if let Some(scroll_bar) = self.horizontal_scrollbar.as_horizontal_scrollbar() {
                let offset = scroll_bar.scroll_offset();
                let range = offset..offset + self.dim.width;
                buffer.copy_horizontal_range_into(
                    self.dim.buffer_x,
                    self.dim.buffer_y,
                    &self.list_buffer,
                    range,
                );
            }
        } else if let Some(scroll_bar) = self.horizontal_scrollbar.as_horizontal_scrollbar() {
            let range = 0..scroll_bar.total_width();
            buffer.copy_vertical_range_into(
                self.dim.buffer_x,
                self.dim.buffer_y,
                &self.list_buffer,
                range,
            );
        }

        self.is_dirty = false;
    }

    /// Convert to the list layout trait
    fn as_rowlist_layout(&mut self) -> Option<&mut dyn TheRowListLayoutTrait> {
        Some(self)
    }
}

/// TheListLayout specific functions.
pub trait TheRowListLayoutTrait: TheLayout {
    /// Adds an item.
    fn add_item(&mut self, item: TheRowListItem, ctx: &mut TheContext);
    /// A new item was selected, manage the selection states.
    fn new_item_selected(&mut self, item: TheId);
    /// Remove all items.
    fn clear(&mut self);
    /// Remove the given list item from the list.
    fn remove(&mut self, id: TheId);
    /// Deselect all items.
    fn deselect_all(&mut self);
    /// Returns the id of the selected item (if any).
    fn selected(&self) -> Option<TheId>;
    /// Set the height of the items
    fn set_item_size(&mut self, item_size: i32);
    /// Selects the first item (and sends events)
    fn select_first_item(&mut self, ctx: &mut TheContext);
    /// Selects the item of the given uuid.
    fn select_item(&mut self, uuid: Uuid, ctx: &mut TheContext, send_event: bool) -> bool;
    /// Selects the item at the given index.
    fn select_item_at(&mut self, index: i32, ctx: &mut TheContext, send_event: bool) -> bool;
    /// Sets the text for an item
    fn set_item_text(&mut self, id: Uuid, text: String);
    /// Sets the image for an item.
    fn set_item_image(&mut self, id: Uuid, image: TheRGBABuffer);
    /// Scroll by the given amount.
    fn scroll_by(&mut self, delta: Vec2<i32>);
}

impl TheRowListLayoutTrait for TheRowListLayout {
    fn add_item(&mut self, mut item: TheRowListItem, ctx: &mut TheContext) {
        item.set_associated_layout(self.id().clone());
        self.widgets.push(Box::new(item));
        ctx.ui.relayout = true;
    }

    fn new_item_selected(&mut self, item: TheId) {
        for w in &mut self.widgets {
            if !w.id().equals(&Some(item.clone())) {
                w.set_state(TheWidgetState::None);
            }
        }
    }

    fn select_first_item(&mut self, ctx: &mut TheContext) {
        self.deselect_all();
        if !self.widgets.is_empty() {
            self.widgets[0].set_state(TheWidgetState::Selected);
            ctx.ui
                .send_widget_state_changed(self.widgets[0].id(), TheWidgetState::Selected);
        }
    }

    fn select_item(&mut self, uuid: Uuid, ctx: &mut TheContext, send_event: bool) -> bool {
        self.deselect_all();
        for w in &mut self.widgets {
            if w.id().uuid == uuid {
                w.set_state(TheWidgetState::Selected);
                if send_event {
                    ctx.ui
                        .send_widget_state_changed(w.id(), TheWidgetState::Selected);
                }
                return true;
            }
        }
        false
    }

    fn select_item_at(&mut self, index: i32, ctx: &mut TheContext, send_event: bool) -> bool {
        self.deselect_all();
        if let Some(w) = self.widgets.get_mut(index as usize) {
            w.set_state(TheWidgetState::Selected);
            if send_event {
                ctx.ui
                    .send_widget_state_changed(w.id(), TheWidgetState::Selected);
            }
            return true;
        }
        false
    }

    fn clear(&mut self) {
        self.widgets.clear();
        self.is_dirty = true;
    }

    fn remove(&mut self, id: TheId) {
        self.widgets.retain(|item| *item.id() != id);
        self.is_dirty = true;
    }

    fn deselect_all(&mut self) {
        for w in &mut self.widgets {
            w.set_state(TheWidgetState::None);
        }
        self.is_dirty = true;
    }

    fn selected(&self) -> Option<TheId> {
        for w in &self.widgets {
            if w.state() == TheWidgetState::Selected {
                return Some(w.id().clone());
            }
        }
        None
    }
    fn set_item_size(&mut self, item_size: i32) {
        self.item_size = item_size;
        self.is_dirty = true;
    }
    fn set_item_text(&mut self, id: Uuid, text: String) {
        for w in &mut self.widgets {
            if w.id().uuid == id {
                w.set_value(TheValue::Text(text.clone()));
                self.is_dirty = true;
            }
        }
    }
    fn set_item_image(&mut self, id: Uuid, image: TheRGBABuffer) {
        for w in &mut self.widgets {
            if w.id().uuid == id {
                w.set_value(TheValue::Image(image.clone()));
                self.is_dirty = true;
            }
        }
    }
    fn scroll_by(&mut self, delta: Vec2<i32>) {
        if let Some(scroll_bar) = self.horizontal_scrollbar.as_horizontal_scrollbar() {
            scroll_bar.scroll_by(-delta.x);
        }
    }
}
