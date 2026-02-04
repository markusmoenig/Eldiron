use crate::prelude::*;

const LIST_RIGHT_MARGIN: i32 = 1;
const LIST_BOTTOM_MARGIN: i32 = 2;

pub struct TheListLayout {
    id: TheId,
    limiter: TheSizeLimiter,

    dim: TheDim,

    widgets: Vec<Box<dyn TheWidget>>,

    list_buffer: TheRGBABuffer,

    vertical_scrollbar: Box<dyn TheWidget>,
    vertical_scrollbar_visible: bool,

    margin: Vec4<i32>,

    background: Option<TheThemeColors>,
    item_size: i32,

    is_dirty: bool,
}

impl TheLayout for TheListLayout {
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

            vertical_scrollbar: Box::new(TheVerticalScrollbar::new(TheId::named(
                "Vertical Scrollbar",
            ))),
            vertical_scrollbar_visible: false,

            margin: Vec4::new(0, 0, 0, 0),

            background: Some(TextLayoutBackground),
            item_size: 17,

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

    fn supports_mouse_wheel(&self) -> bool {
        true
    }

    fn mouse_wheel_scroll(&mut self, delta: Vec2<i32>) {
        if let Some(scroll_bar) = self.vertical_scrollbar.as_vertical_scrollbar() {
            scroll_bar.scroll_by(-delta.y);
        }
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

            let x = 1;
            let mut y = 1;
            let mut width = dim.width;

            let items = self.widgets.len() as i32;
            let mut total_height = 1 + items * self.item_size + 1;
            if items > 0 {
                total_height += (items - 1) * 3;
            }
            total_height += LIST_BOTTOM_MARGIN;

            if total_height < dim.height {
                total_height = dim.height;
            }

            self.vertical_scrollbar
                .set_dim(TheDim::new(dim.x + width - 13, dim.y, 13, dim.height), ctx);
            self.vertical_scrollbar
                .dim_mut()
                .set_buffer_offset(self.dim.buffer_x + width - 13, self.dim.buffer_y);

            if let Some(scroll_bar) = self.vertical_scrollbar.as_vertical_scrollbar() {
                scroll_bar.set_total_height(total_height);
                self.vertical_scrollbar_visible = scroll_bar.needs_scrollbar();
            }

            if self.vertical_scrollbar_visible {
                width -= 13;
            }

            let content_width = (width - LIST_RIGHT_MARGIN).max(0);

            self.list_buffer
                .set_dim(TheDim::new(0, 0, width, total_height));

            for index in 0..items {
                let i = index as usize;

                let widget_width = (content_width - 2).max(0);
                self.widgets[i].set_dim(
                    TheDim::new(dim.x + x, dim.y + y, widget_width, self.item_size),
                    ctx,
                );
                self.widgets[i].dim_mut().set_buffer_offset(x, y);

                y += self.item_size + 3;
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

        if self.vertical_scrollbar_visible {
            self.vertical_scrollbar.draw(buffer, style, ctx);
        }

        let mut offset = 0;

        if self.vertical_scrollbar_visible {
            if let Some(scroll_bar) = self.vertical_scrollbar.as_vertical_scrollbar() {
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

        self.is_dirty = false;
    }

    /// Convert to the list layout trait
    fn as_list_layout(&mut self) -> Option<&mut dyn TheListLayoutTrait> {
        Some(self)
    }
}

/// TheListLayout specific functions.
pub trait TheListLayoutTrait: TheLayout {
    /// Adds an item.
    fn add_item(&mut self, item: TheListItem, ctx: &mut TheContext);
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
    /// Sets the icon for an item.
    fn set_item_icon(&mut self, id: Uuid, image: TheRGBABuffer);
    /// Selects the first item (and sends events)
    fn select_first_item(&mut self, ctx: &mut TheContext);
    /// Selects the item of the given uuid.
    fn select_item(&mut self, uuid: Uuid, ctx: &mut TheContext, send_event: bool) -> bool;
    /// Selects the item at the given index.
    fn select_item_at(&mut self, index: i32, ctx: &mut TheContext, send_event: bool) -> bool;
    /// Sets the text for an item
    fn set_item_text(&mut self, id: Uuid, text: String);
    /// Scroll by the given amount.
    fn scroll_by(&mut self, delta: Vec2<i32>);
}

impl TheListLayoutTrait for TheListLayout {
    fn add_item(&mut self, mut item: TheListItem, ctx: &mut TheContext) {
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
    fn set_item_icon(&mut self, id: Uuid, image: TheRGBABuffer) {
        for w in &mut self.widgets {
            if w.id().uuid == id {
                w.set_value(TheValue::Image(image.clone()));
                self.is_dirty = true;
            }
        }
    }
    fn scroll_by(&mut self, delta: Vec2<i32>) {
        if let Some(scroll_bar) = self.vertical_scrollbar.as_vertical_scrollbar() {
            scroll_bar.scroll_by(-delta.y);
        }
    }
}
