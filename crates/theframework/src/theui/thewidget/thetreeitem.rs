use crate::prelude::*;

pub struct TheTreeItem {
    id: TheId,
    limiter: TheSizeLimiter,

    state: TheWidgetState,

    text: String,
    sub_text: String,

    dim: TheDim,
    is_dirty: bool,

    mouse_down_pos: Vec2<i32>,
    mouse_down_in_widget: bool,

    icon: Option<TheRGBABuffer>,
    status: Option<String>,

    layout_id: TheId,
    scroll_offset: i32,

    values: Vec<(i32, TheValue)>,
    widget_column: Option<(i32, Box<dyn TheWidget>)>,

    context_menu: Option<TheContextMenu>,
    cursor_icon: Option<TheCursorIcon>,

    background: Option<TheColor>,
}

impl TheWidget for TheTreeItem {
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

            text: "".to_string(),
            sub_text: "".to_string(),

            dim: TheDim::zero(),
            is_dirty: true,
            mouse_down_pos: Vec2::zero(),
            mouse_down_in_widget: false,

            icon: None,
            status: None,

            layout_id: TheId::empty(),
            scroll_offset: 0,

            values: Vec::new(),
            widget_column: None,

            context_menu: None,
            cursor_icon: None,

            background: None,
        }
    }

    fn id(&self) -> &TheId {
        &self.id
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
            TheEvent::MouseDown(coord) => {
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

                // Track if mouse down happened in embedded widget
                self.mouse_down_in_widget = false;
                if let Some((_width, w)) = &mut self.widget_column {
                    let dim = w.dim();
                    let widget_coord =
                        Vec2::new(coord.x - dim.x, coord.y - dim.y + self.scroll_offset);

                    // Check if the click is within the embedded widget bounds
                    if widget_coord.x >= 0
                        && widget_coord.y >= 0
                        && widget_coord.x < dim.width
                        && widget_coord.y < dim.height
                    {
                        self.mouse_down_in_widget = true;
                        redraw = w.on_event(&TheEvent::MouseDown(widget_coord), ctx);
                    }
                }

                self.mouse_down_pos = Vec2::new(coord.x, coord.y + self.scroll_offset);
            }
            TheEvent::MouseUp(coord) => {
                // If mouse down was in widget, forward mouse up (even if outside bounds)
                if self.mouse_down_in_widget {
                    if let Some((_, w)) = &mut self.widget_column {
                        let dim = w.dim();
                        let widget_coord =
                            Vec2::new(coord.x - dim.x, coord.y - dim.y + self.scroll_offset);
                        redraw = w.on_event(&TheEvent::MouseUp(widget_coord), ctx);
                        self.is_dirty = true;
                    }
                    self.mouse_down_in_widget = false;
                } else if let Some((_, w)) = &mut self.widget_column {
                    let dim = w.dim();
                    let widget_coord =
                        Vec2::new(coord.x - dim.x, coord.y - dim.y + self.scroll_offset);

                    // Check if the click is within the embedded widget bounds
                    if widget_coord.x >= 0
                        && widget_coord.y >= 0
                        && widget_coord.x < dim.width
                        && widget_coord.y < dim.height
                    {
                        redraw = w.on_event(&TheEvent::MouseUp(widget_coord), ctx);
                    }
                    self.is_dirty = true;
                }
            }
            TheEvent::Cut => {
                if let Some((_, w)) = &mut self.widget_column {
                    redraw = w.on_event(event, ctx);
                }
            }
            TheEvent::Copy => {
                if let Some((_, w)) = &mut self.widget_column {
                    redraw = w.on_event(event, ctx);
                }
            }
            TheEvent::Paste(value, app_type) => {
                if let Some((_, w)) = &mut self.widget_column {
                    redraw = w.on_event(&TheEvent::Paste(value.clone(), app_type.clone()), ctx);
                }
            }
            TheEvent::MouseDragged(coord) => {
                // If mouse down happened in embedded widget, always forward drags to it
                let mut handled_by_widget = false;
                if self.mouse_down_in_widget {
                    if let Some((_, w)) = &mut self.widget_column {
                        let dim = w.dim();
                        let widget_coord =
                            Vec2::new(coord.x - dim.x, coord.y - dim.y + self.scroll_offset);

                        // Always forward drag events if mouse down was in widget (even if mouse is now outside)
                        redraw = w.on_event(&TheEvent::MouseDragged(widget_coord), ctx);
                        handled_by_widget = true;
                    }
                }

                // Only handle drag for tree item if not handled by embedded widget
                if !handled_by_widget {
                    let coord = Vec2::new(coord.x, coord.y + self.scroll_offset);
                    if ctx.ui.drop.is_none()
                        && Vec2::new(self.mouse_down_pos.x as f32, self.mouse_down_pos.y as f32)
                            .distance(Vec2::new(coord.x as f32, coord.y as f32))
                            >= 5.0
                    {
                        let mut text = self.text.clone();
                        if let Some((_, w)) = &mut self.widget_column {
                            if let TheValue::Text(t) = w.value() {
                                text = t.clone();
                            }
                        }
                        ctx.ui
                            .send(TheEvent::DragStarted(self.id().clone(), text, coord));
                    }
                }
            }
            TheEvent::Hover(coord) => {
                if self.state != TheWidgetState::Selected && !self.id().equals(&ctx.ui.hover) {
                    self.is_dirty = true;
                    ctx.ui.set_hover(self.id());
                    redraw = true;
                }

                // Pass hover events to embedded widget
                if let Some((_, w)) = &mut self.widget_column {
                    let dim = w.dim();
                    let widget_coord = Vec2::new(coord.x - dim.x, coord.y - dim.y);

                    // Check if the hover is within the embedded widget bounds
                    if widget_coord.x >= 0
                        && widget_coord.y >= 0
                        && widget_coord.x < dim.width
                        && widget_coord.y < dim.height
                    {
                        w.on_event(&TheEvent::Hover(widget_coord), ctx);

                        // Update cursor icon based on embedded widget
                        if let Some(cursor_icon) = w.cursor_icon() {
                            self.cursor_icon = Some(cursor_icon);
                        }
                    } else {
                        // Reset cursor icon when not hovering embedded widget
                        self.cursor_icon = None;
                    }
                } else {
                    // Reset cursor icon when no embedded widget
                    self.cursor_icon = None;
                }
            }
            TheEvent::MouseWheel(delta) => {
                ctx.ui
                    .send(TheEvent::ScrollLayout(self.layout_id.clone(), *delta));
            }
            _ => {
                // Only pass specific events to embedded widget that don't depend on mouse position
                // This prevents embedded widgets from receiving events that should only go to the tree item
                let has_focus = ctx.ui.has_focus(self.id());
                if let Some((_, w)) = &mut self.widget_column {
                    match event {
                        // Pass focus events to embedded widget
                        TheEvent::GainedFocus(_) | TheEvent::LostFocus(_) => {
                            redraw = w.on_event(event, ctx);
                        }
                        // Pass keyboard events to embedded widget when parent tree item has focus
                        TheEvent::KeyDown(_)
                        | TheEvent::KeyUp(_)
                        | TheEvent::KeyCodeDown(_)
                        | TheEvent::KeyCodeUp(_) => {
                            if has_focus {
                                redraw = w.on_event(event, ctx);
                            }
                        }
                        // Pass modifier changes to embedded widget when parent tree item has focus
                        TheEvent::ModifierChanged(_, _, _, _) => {
                            if has_focus {
                                redraw = w.on_event(event, ctx);
                            }
                        }

                        // Pass undo/redo events to embedded widget when parent tree item has focus
                        TheEvent::Undo | TheEvent::Redo => {
                            if has_focus {
                                redraw = w.on_event(event, ctx);
                            }
                        }
                        // Don't pass other events to prevent unwanted value changes
                        _ => {}
                    }
                }
            }
        }
        redraw
    }

    fn dim(&self) -> &TheDim {
        &self.dim
    }

    fn dim_mut(&mut self) -> &mut TheDim {
        &mut self.dim
    }

    fn set_dim(&mut self, dim: TheDim, ctx: &mut TheContext) {
        if self.dim != dim {
            self.dim = dim;
            self.is_dirty = true;

            // Set dimension for embedded widget column
            if let Some((width, widget)) = &mut self.widget_column {
                widget.calculate_size(ctx);
                let height = widget.limiter().get_max_height();
                let y = (22 - height) / 2;

                // Position widget at the right side with +9 offset (matching draw method)
                let widget_x = self.dim.width - *width;
                widget.set_dim(
                    TheDim::new(widget_x + 9, y, *width as i32 - 10, height),
                    ctx,
                );
            }
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

    fn supports_text_input(&self) -> bool {
        if let Some((_, widget)) = &self.widget_column {
            return widget.supports_text_input();
        }
        false
    }

    fn cursor_icon(&self) -> Option<TheCursorIcon> {
        // Return cursor icon from embedded widget if available, otherwise None
        if let Some(cursor_icon) = self.cursor_icon {
            Some(cursor_icon)
        } else if let Some((_, widget)) = &self.widget_column {
            widget.cursor_icon()
        } else {
            None
        }
    }

    fn set_cursor_icon(&mut self, icon: Option<TheCursorIcon>) {
        self.cursor_icon = icon;
    }

    fn supports_clipboard(&mut self) -> bool {
        if let Some((_, widget)) = &mut self.widget_column {
            widget.supports_clipboard()
        } else {
            false
        }
    }

    fn value(&self) -> TheValue {
        TheValue::Text(self.text.clone())
    }

    fn set_value(&mut self, value: TheValue) {
        match value {
            TheValue::Empty => {
                self.text = "".to_string();
                self.is_dirty = true;
            }
            TheValue::Text(text) => {
                self.text.clone_from(&text);
                self.is_dirty = true;
            }
            TheValue::Image(image) => {
                self.icon = Some(image);
                self.is_dirty = true;
            }
            _ => {}
        }
    }

    fn status_text(&self) -> Option<String> {
        self.status.clone()
    }

    fn set_status_text(&mut self, text: &str) {
        self.status = Some(text.to_string());
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
        // Use buffer coordinates (which are relative to the content buffer)
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

        let mut color = if self.state == TheWidgetState::Selected {
            if !self.id().equals(&ctx.ui.focus) {
                *style.theme().color(ListItemSelectedNoFocus)
            } else {
                *style.theme().color(ListItemSelected)
            }
        } else if let Some(background) = &self.background {
            background.to_u8_array()
        } else {
            *style.theme().color(ListItemNormal)
        };

        if self.state != TheWidgetState::Selected && self.id().equals(&ctx.ui.hover) {
            color = *style.theme().color(ListItemHover)
        }

        let stride = buffer.stride();
        let mut shrinker = TheDimShrinker::zero();

        // Safety check: ensure tree item is within buffer bounds before drawing outline
        // Adjust for transparent top/bottom areas: draw 1px lower and 2px shorter
        let mut adjusted_utuple = self.dim.to_buffer_shrunk_utuple(&shrinker);
        adjusted_utuple.1 += 1; // Draw 1px lower
        adjusted_utuple.3 = adjusted_utuple.3.saturating_sub(2); // Reduce height by 2px
        let buffer_width = buffer.dim().width as usize;
        let buffer_height = buffer.dim().height as usize;

        // Additional defensive check: ensure we don't draw outside buffer bounds
        if adjusted_utuple.0 < buffer_width
            && adjusted_utuple.1 < buffer_height
            && adjusted_utuple.0 + adjusted_utuple.2 <= buffer_width
            && adjusted_utuple.1 + adjusted_utuple.3 <= buffer_height
        {
            ctx.draw.rect_outline_border_open(
                buffer.pixels_mut(),
                &adjusted_utuple,
                stride,
                &color,
                1,
            );
        }

        shrinker.shrink(1);
        // Safety check: ensure tree item is within buffer bounds before drawing fill
        // Adjust for transparent top/bottom areas: draw 1px lower and 2px shorter
        let mut adjusted_utuple = self.dim.to_buffer_shrunk_utuple(&shrinker);
        adjusted_utuple.1 += 1; // Draw 1px lower
        adjusted_utuple.3 = adjusted_utuple.3.saturating_sub(2); // Reduce height by 2px
        let buffer_width = buffer.dim().width as usize;
        let buffer_height = buffer.dim().height as usize;

        // Additional defensive check: ensure we don't draw outside buffer bounds
        if adjusted_utuple.0 < buffer_width
            && adjusted_utuple.1 < buffer_height
            && adjusted_utuple.0 + adjusted_utuple.2 <= buffer_width
            && adjusted_utuple.1 + adjusted_utuple.3 <= buffer_height
        {
            ctx.draw
                .rect(buffer.pixels_mut(), &adjusted_utuple, stride, &color);
        }

        if let Some(icon) = &self.icon {
            let ut = self.dim.to_buffer_shrunk_utuple(&shrinker);
            let icon_rect = (ut.0 + 1, ut.1 + 2, 38, 38); // Adjust Y position by +1px
            let buffer_width = buffer.dim().width as usize;
            let buffer_height = buffer.dim().height as usize;

            // Safety check: ensure icon is within buffer bounds before drawing
            if icon_rect.0 < buffer_width
                && icon_rect.1 < buffer_height
                && icon_rect.0 + icon_rect.2 <= buffer_width
                && icon_rect.1 + icon_rect.3 <= buffer_height
            {
                ctx.draw.rect_outline_border(
                    buffer.pixels_mut(),
                    &icon_rect,
                    stride,
                    style.theme().color(ListItemIconBorder),
                    1,
                );
            }
            let icon_copy_rect = (ut.0 + 2, ut.1 + 3, 36, 36); // Adjust Y position by +1px
            let buffer_width = buffer.dim().width as usize;
            let buffer_height = buffer.dim().height as usize;

            // Safety check: ensure icon copy is within buffer bounds
            if icon_copy_rect.0 < buffer_width
                && icon_copy_rect.1 < buffer_height
                && icon_copy_rect.0 + icon_copy_rect.2 <= buffer_width
                && icon_copy_rect.1 + icon_copy_rect.3 <= buffer_height
            {
                ctx.draw
                    .copy_slice(buffer.pixels_mut(), icon.pixels(), &icon_copy_rect, stride);
            }

            let text_rect = (
                ut.0 + 38 + 7 + 5,
                ut.1 + 6, // Adjust Y position by +1px
                (self.dim.width - 38 - 7 - 10) as usize,
                13,
            );
            let buffer_width = buffer.dim().width as usize;
            let buffer_height = buffer.dim().height as usize;

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
                    &self.text,
                    TheFontSettings {
                        size: 12.0,
                        ..Default::default()
                    },
                    style.theme().color(ListItemText),
                    TheHorizontalAlign::Left,
                    TheVerticalAlign::Center,
                );
            }

            if !self.sub_text.is_empty() {
                let sub_text_rect = (
                    ut.0 + 38 + 7 + 5,
                    ut.1 + 23, // Adjust Y position by +1px
                    (self.dim.width - 38 - 7 - 10) as usize,
                    13,
                );
                let buffer_width = buffer.dim().width as usize;
                let buffer_height = buffer.dim().height as usize;

                // Safety check: ensure sub text is within buffer bounds before drawing
                if sub_text_rect.0 < buffer_width
                    && sub_text_rect.1 < buffer_height
                    && sub_text_rect.0 + sub_text_rect.2 <= buffer_width
                    && sub_text_rect.1 + sub_text_rect.3 <= buffer_height
                {
                    ctx.draw.text_rect_blend(
                        buffer.pixels_mut(),
                        &sub_text_rect,
                        stride,
                        &self.sub_text,
                        TheFontSettings {
                            size: 12.0,
                            ..Default::default()
                        },
                        style.theme().color(ListItemText),
                        TheHorizontalAlign::Left,
                        TheVerticalAlign::Center,
                    );
                }
            }
        } else {
            let mut right_width = 5;
            for v in self.values.iter() {
                right_width += v.0;
            }
            if let Some((width, _)) = &self.widget_column {
                right_width += *width;
            }

            shrinker.shrink_by(9, 0, 0, 0);
            let mut rect: (usize, usize, usize, usize) =
                self.dim.to_buffer_shrunk_utuple(&shrinker);

            let text_rect = (
                rect.0,
                rect.1 + 1,
                rect.2 - right_width as usize,
                rect.3.saturating_sub(2),
            ); // Adjust Y position by +1px and reduce height by 2px
            let buffer_width = buffer.dim().width as usize;
            let buffer_height = buffer.dim().height as usize;

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
                    &self.text,
                    TheFontSettings {
                        size: 13.0,
                        ..Default::default()
                    },
                    style.theme().color(ListItemText),
                    TheHorizontalAlign::Left,
                    TheVerticalAlign::Center,
                );
            }

            rect.0 += rect.2 - right_width as usize;

            if let Some((_width, widget)) = &mut self.widget_column {
                ctx.draw.rect(
                    buffer.pixels_mut(),
                    &(rect.0, rect.1 - 1, 1, rect.3 + 2),
                    stride,
                    style.theme().color(ListLayoutBackground),
                );

                // Set buffer offset for drawing (dimension should already be set in set_dim)
                let y_offset = rect.1 as i32 + (22 - widget.dim().height) / 2;
                widget
                    .dim_mut()
                    .set_buffer_offset(rect.0 as i32 + 9, y_offset);
                widget.draw(buffer, style, ctx);
            }

            for (width, value) in self.values.iter() {
                ctx.draw.rect(
                    buffer.pixels_mut(),
                    &(rect.0, rect.1 - 1, 1, rect.3 + 2),
                    stride,
                    style.theme().color(ListLayoutBackground),
                );

                #[allow(clippy::single_match)]
                match value {
                    TheValue::Text(text) => {
                        let value_rect = (
                            rect.0 + 9,
                            rect.1 + 1,
                            *width as usize - 10,
                            rect.3.saturating_sub(2),
                        ); // Adjust Y position by +1px and reduce height by 2px
                        let buffer_width = buffer.dim().width as usize;
                        let buffer_height = buffer.dim().height as usize;

                        // Safety check: ensure value text is within buffer bounds before drawing
                        if value_rect.0 < buffer_width
                            && value_rect.1 < buffer_height
                            && value_rect.0 + value_rect.2 <= buffer_width
                            && value_rect.1 + value_rect.3 <= buffer_height
                        {
                            ctx.draw.text_rect_blend(
                                buffer.pixels_mut(),
                                &value_rect,
                                stride,
                                text,
                                TheFontSettings {
                                    size: 13.0,
                                    ..Default::default()
                                },
                                style.theme().color(ListItemText),
                                TheHorizontalAlign::Left,
                                TheVerticalAlign::Center,
                            );
                        }
                    }
                    _ => {
                        let value_rect = (
                            rect.0 + 9,
                            rect.1 + 1,
                            *width as usize - 10,
                            rect.3.saturating_sub(2),
                        ); // Adjust Y position by +1px and reduce height by 2px
                        let buffer_width = buffer.dim().width as usize;
                        let buffer_height = buffer.dim().height as usize;

                        // Safety check: ensure value text is within buffer bounds before drawing
                        if value_rect.0 < buffer_width
                            && value_rect.1 < buffer_height
                            && value_rect.0 + value_rect.2 <= buffer_width
                            && value_rect.1 + value_rect.3 <= buffer_height
                        {
                            ctx.draw.text_rect_blend(
                                buffer.pixels_mut(),
                                &value_rect,
                                stride,
                                &value.describe(),
                                TheFontSettings {
                                    size: 13.0,
                                    ..Default::default()
                                },
                                style.theme().color(ListItemText),
                                TheHorizontalAlign::Left,
                                TheVerticalAlign::Center,
                            );
                        }
                    }
                }
            }
        }

        self.is_dirty = false;
    }

    fn as_tree_item(&mut self) -> Option<&mut dyn TheTreeItemTrait> {
        Some(self)
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn draw_overlay(
        &mut self,
        style: &mut Box<dyn TheStyle>,
        ctx: &mut TheContext,
    ) -> TheRGBABuffer {
        if let Some((_, widget)) = &mut self.widget_column {
            let mut buffer = widget.draw_overlay(style, ctx);
            let d = buffer.dim_mut();
            // d.x += self.dim().x;
            // d.y += self.dim().y;
            d.buffer_x = d.x + self.dim().x - 6;
            d.buffer_y = d.y + self.dim().y + 1 - self.scroll_offset;

            buffer
        } else {
            TheRGBABuffer::default()
        }
    }
}

pub trait TheTreeItemTrait {
    fn set_background_color(&mut self, color: TheColor);
    fn set_text(&mut self, text: String);
    fn set_sub_text(&mut self, sub_text: String);
    fn set_associated_layout(&mut self, id: TheId);
    fn set_size(&mut self, size: i32);
    fn set_icon(&mut self, icon: TheRGBABuffer);
    fn set_scroll_offset(&mut self, offset: i32);
    fn add_value_column(&mut self, width: i32, value: TheValue);
    fn add_widget_column(&mut self, width: i32, value: Box<dyn TheWidget>);
    fn embedded_widget_mut(&mut self) -> Option<&mut Box<dyn TheWidget>>;
}

impl TheTreeItemTrait for TheTreeItem {
    fn set_background_color(&mut self, color: TheColor) {
        self.background = Some(color);
        self.is_dirty = true;
    }
    fn set_text(&mut self, text: String) {
        self.text = text;
        self.is_dirty = true;
    }
    fn set_sub_text(&mut self, sub_text: String) {
        self.sub_text = sub_text;
        self.is_dirty = true;
    }
    fn set_associated_layout(&mut self, layout_id: TheId) {
        self.layout_id = layout_id;
    }
    fn set_size(&mut self, size: i32) {
        self.limiter_mut().set_max_height(size);
        self.is_dirty = true;
    }
    fn set_icon(&mut self, icon: TheRGBABuffer) {
        self.icon = Some(icon);
    }
    fn set_scroll_offset(&mut self, offset: i32) {
        self.scroll_offset = offset;
    }
    fn add_value_column(&mut self, width: i32, value: TheValue) {
        self.values.push((width, value));
    }
    fn add_widget_column(&mut self, width: i32, mut widget: Box<dyn TheWidget>) {
        widget.set_embedded(true);
        widget.set_parent_id(self.id.clone());
        self.widget_column = Some((width, widget));
    }
    fn embedded_widget_mut(&mut self) -> Option<&mut Box<dyn TheWidget>> {
        self.widget_column.as_mut().map(|(_, widget)| widget)
    }
}
