use crate::prelude::*;

pub struct TheTreeIcons {
    id: TheId,
    limiter: TheSizeLimiter,

    state: TheWidgetState,

    dim: TheDim,
    is_dirty: bool,

    layout_id: TheId,
    scroll_offset: i32,

    icons: Vec<Option<TheRGBABuffer>>,
    texts: Vec<Option<String>>,
    status_texts: Vec<Option<String>>,
    selected_index: Option<usize>,
    hovered_index: Option<usize>,

    icon_size: i32,
    icons_per_row: i32,
    spacing: i32,

    rectangles: Vec<TheDim>,

    context_menu: Option<TheContextMenu>,
}

impl TheWidget for TheTreeIcons {
    fn new(id: TheId) -> Self
    where
        Self: Sized,
    {
        let mut limiter = TheSizeLimiter::new();
        limiter.set_max_height(100);

        Self {
            id,
            limiter,

            state: TheWidgetState::None,

            dim: TheDim::zero(),
            is_dirty: true,

            layout_id: TheId::empty(),
            scroll_offset: 0,

            icons: vec![],
            texts: vec![],
            status_texts: vec![],
            selected_index: None,
            hovered_index: None,

            icon_size: 18,
            icons_per_row: 10,
            spacing: 1,

            rectangles: vec![],

            context_menu: None,
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
            TheEvent::Drop(coord, drop) => {
                if drop.id.name == "Tile" {
                    // Adjust coordinates for scroll offset from layout
                    let adjusted_coord = Vec2::new(coord.x, coord.y + self.scroll_offset);
                    // Find which icon was dropped on
                    for (i, rect) in self.rectangles.iter().enumerate() {
                        if rect.contains(adjusted_coord) && i < self.icons.len() {
                            let tile_id = drop.id.uuid;
                            ctx.ui
                                .send(TheEvent::TileDropped(self.id().clone(), tile_id, i));
                            self.is_dirty = true;
                            return true;
                        }
                    }
                }
            }
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
                // Adjust coordinates for scroll offset from layout
                let adjusted_coord = Vec2::new(coord.x, coord.y + self.scroll_offset);

                // Check if clicking on an icon
                let mut clicked_icon = false;
                for (i, rect) in self.rectangles.iter().enumerate() {
                    if rect.contains(adjusted_coord) && i < self.icons.len() {
                        self.selected_index = Some(i);
                        ctx.ui.send(TheEvent::IndexChanged(self.id().clone(), i));
                        clicked_icon = true;
                        self.is_dirty = true;
                        redraw = true;
                        break;
                    }
                }

                // Only set widget state as selected if we clicked on an icon
                if clicked_icon {
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
            }
            TheEvent::Hover(coord) => {
                // Adjust coordinates for scroll offset from layout
                let adjusted_coord = Vec2::new(coord.x, coord.y + self.scroll_offset);

                let mut new_hover = None;
                for (i, rect) in self.rectangles.iter().enumerate() {
                    if rect.contains(adjusted_coord) && i < self.icons.len() {
                        new_hover = Some(i);
                        break;
                    }
                }

                if new_hover != self.hovered_index {
                    self.hovered_index = new_hover;

                    // Send status text update event
                    if let Some(index) = new_hover {
                        if index < self.status_texts.len() {
                            if let Some(status_text) = &self.status_texts[index] {
                                ctx.ui.send(TheEvent::SetStatusText(
                                    self.id().clone(),
                                    status_text.clone(),
                                ));
                            } else {
                                ctx.ui.send(TheEvent::SetStatusText(
                                    self.id().clone(),
                                    "".to_string(),
                                ));
                            }
                        }
                    } else {
                        ctx.ui
                            .send(TheEvent::SetStatusText(self.id().clone(), "".to_string()));
                    }

                    self.is_dirty = true;
                    redraw = true;
                }

                // Set hover state on the widget itself
                if !self.id().equals(&ctx.ui.hover) {
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
        if self.dim != dim {
            self.dim = dim;
            self.is_dirty = true;
            self.update_height();
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

    fn status_text(&self) -> Option<String> {
        if let Some(index) = self.hovered_index {
            if index < self.status_texts.len() {
                return self.status_texts[index].clone();
            }
        }
        None
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

        // Always use normal background color - selection is shown per-icon
        let color = *style.theme().color(ListItemNormal);

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
                &color,
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
                .rect(buffer.pixels_mut(), &adjusted_utuple, stride, &color);
        }

        // Draw icons in a grid
        self.rectangles.clear();

        let start_x = 9;
        let start_y = 5;
        let mut x_off = start_x;
        let mut y_off = start_y;
        let mut col = 0;

        for (index, icon_opt) in self.icons.iter().enumerate() {
            // Buffer coordinates for drawing
            let icon_rect = (
                adjusted_utuple.0 + x_off,
                adjusted_utuple.1 + y_off,
                self.icon_size as usize,
                self.icon_size as usize,
            );

            // Widget-relative coordinates for hit testing
            // Events use logical widget coordinates (dim.x, dim.y based)
            // NOT buffer coordinates, so just use the raw offsets
            let hit_rect = TheDim::new(x_off as i32, y_off as i32, self.icon_size, self.icon_size);
            self.rectangles.push(hit_rect);

            // Safety check: only draw if icon is within buffer bounds
            if icon_rect.0 < buffer_width
                && icon_rect.1 < buffer_height
                && icon_rect.0 + icon_rect.2 <= buffer_width
                && icon_rect.1 + icon_rect.3 <= buffer_height
            {
                // Draw selection/hover highlight
                if Some(index) == self.selected_index {
                    ctx.draw
                        .rect_outline(buffer.pixels_mut(), &icon_rect, stride, &WHITE);
                } else if Some(index) == self.hovered_index {
                    let mut hover_color = *style.theme().color(ListItemHover);
                    hover_color[3] = 128; // Semi-transparent
                    ctx.draw
                        .rect_outline(buffer.pixels_mut(), &icon_rect, stride, &hover_color);
                }

                // Draw icon border
                ctx.draw.rect_outline(
                    buffer.pixels_mut(),
                    &(
                        icon_rect.0 + 1,
                        icon_rect.1 + 1,
                        icon_rect.2.saturating_sub(2),
                        icon_rect.3.saturating_sub(2),
                    ),
                    stride,
                    &BLACK,
                );

                // Draw icon content
                if let Some(icon) = icon_opt {
                    // Only draw if the icon matches the expected size
                    if icon.dim().width == self.icon_size - 4
                        && icon.dim().height == self.icon_size - 4
                    {
                        let content_rect = (
                            icon_rect.0 + 2,
                            icon_rect.1 + 2,
                            (self.icon_size - 4) as usize,
                            (self.icon_size - 4) as usize,
                        );
                        ctx.draw.copy_slice(
                            buffer.pixels_mut(),
                            icon.pixels(),
                            &content_rect,
                            stride,
                        );
                    }
                }

                // Draw text overlay (always if present, even over icons)
                if index < self.texts.len() {
                    if let Some(text) = &self.texts[index] {
                        let text_color = WHITE;
                        let font_size = 9.0; // Small font size

                        // Calculate text rectangle to center it in the icon area
                        let text_rect = (
                            icon_rect.0,
                            icon_rect.1,
                            (self.icon_size) as usize,
                            (self.icon_size) as usize,
                        );

                        ctx.draw.text_rect_blend(
                            buffer.pixels_mut(),
                            &text_rect,
                            stride,
                            text,
                            TheFontSettings {
                                size: font_size,
                                ..Default::default()
                            },
                            &text_color,
                            TheHorizontalAlign::Center,
                            TheVerticalAlign::Center,
                        );
                    }
                }
            }

            col += 1;
            if col >= self.icons_per_row {
                col = 0;
                x_off = start_x;
                y_off += self.icon_size as usize + self.spacing as usize;
            } else {
                x_off += self.icon_size as usize + self.spacing as usize;
            }
        }

        self.is_dirty = false;
    }

    fn as_tree_icons(&mut self) -> Option<&mut dyn TheTreeIconsTrait> {
        Some(self)
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl TheTreeIcons {
    /// Calculate and update the height based on the number of icons and layout
    fn update_height(&mut self) {
        if self.icons.is_empty() {
            self.limiter.set_max_height(22);
            return;
        }

        let rows = (self.icons.len() as i32 + self.icons_per_row - 1) / self.icons_per_row;
        // Account for padding: 5px top + 5px bottom = 10px total vertical padding
        // Also account for the drawing adjustments in draw():
        // - shrinker.shrink(1) removes 2px total height (1px top, 1px bottom)
        // - adjusted_utuple.1 += 1 and adjusted_utuple.3 -= 2 removes another 2px
        // Total shrinkage: 4px, so we need to add that back
        let height = 10 + rows * self.icon_size + (rows - 1).max(0) * self.spacing + 4;
        self.limiter.set_max_height(height);
    }
}

pub trait TheTreeIconsTrait {
    fn set_associated_layout(&mut self, id: TheId);
    fn set_scroll_offset(&mut self, offset: i32);
    fn set_icon_size(&mut self, size: i32);
    fn set_icons_per_row(&mut self, count: i32);
    fn set_spacing(&mut self, spacing: i32);
    fn set_icon_count(&mut self, count: usize);
    fn set_icon(&mut self, index: usize, icon: TheRGBABuffer);
    fn set_text(&mut self, index: usize, text: String);
    fn set_status_text_for(&mut self, index: usize, text: String);
    fn set_palette(&mut self, palette: &ThePalette);
    fn clear_icons(&mut self);
    fn selected_index(&self) -> Option<usize>;
    fn set_selected_index(&mut self, index: Option<usize>);
}

impl TheTreeIconsTrait for TheTreeIcons {
    fn set_associated_layout(&mut self, layout_id: TheId) {
        self.layout_id = layout_id;
    }

    fn set_scroll_offset(&mut self, offset: i32) {
        self.scroll_offset = offset;
    }

    fn set_icon_size(&mut self, size: i32) {
        self.icon_size = size;
        self.update_height();
        self.is_dirty = true;
    }

    fn set_icons_per_row(&mut self, count: i32) {
        self.icons_per_row = count;
        self.update_height();
        self.is_dirty = true;
    }

    fn set_spacing(&mut self, spacing: i32) {
        self.spacing = spacing;
        self.update_height();
        self.is_dirty = true;
    }

    fn set_icon_count(&mut self, count: usize) {
        self.icons.resize(count, None);
        self.texts.resize(count, None);
        self.status_texts.resize(count, None);
        self.update_height();
        self.is_dirty = true;
    }

    fn set_icon(&mut self, index: usize, icon: TheRGBABuffer) {
        if index < self.icons.len() {
            let expected_size = self.icon_size - 4;

            // Automatically resize icon if it doesn't match the expected size
            let final_icon =
                if icon.dim().width != expected_size || icon.dim().height != expected_size {
                    icon.scaled(expected_size, expected_size)
                } else {
                    icon
                };

            self.icons[index] = Some(final_icon);
            self.is_dirty = true;
        }
    }

    fn set_text(&mut self, index: usize, text: String) {
        // Ensure texts vec is large enough
        if index >= self.texts.len() {
            self.texts.resize(index + 1, None);
        }
        self.texts[index] = Some(text);
        self.is_dirty = true;
    }

    fn set_status_text_for(&mut self, index: usize, text: String) {
        // Ensure status_texts vec is large enough
        if index >= self.status_texts.len() {
            self.status_texts.resize(index + 1, None);
        }
        self.status_texts[index] = Some(text);
    }

    fn set_palette(&mut self, palette: &ThePalette) {
        self.icons.clear();
        self.texts.clear();
        self.status_texts.clear();

        for (index, color_opt) in palette.colors.iter().enumerate() {
            if let Some(color) = color_opt {
                // Create a small buffer with the palette color
                let mut icon =
                    TheRGBABuffer::new(TheDim::sized(self.icon_size - 4, self.icon_size - 4));
                let color_array = color.to_u8_array();
                for pixel in icon.pixels_mut().chunks_exact_mut(4) {
                    pixel.copy_from_slice(&color_array);
                }
                self.icons.push(Some(icon));

                // Create status text with index and hex color
                let hex_color = format!(
                    "#{:02X}{:02X}{:02X}",
                    color_array[0], color_array[1], color_array[2]
                );
                self.status_texts.push(Some(format!(
                    "Palette Index {}. Color {}",
                    index, hex_color
                )));
            } else {
                self.icons.push(None);
                self.status_texts.push(None);
            }
        }

        self.update_height();
        self.is_dirty = true;
    }

    fn clear_icons(&mut self) {
        self.icons.clear();
        self.texts.clear();
        self.status_texts.clear();
        self.selected_index = None;
        self.hovered_index = None;
        self.update_height();
        self.is_dirty = true;
    }

    fn selected_index(&self) -> Option<usize> {
        self.selected_index
    }

    fn set_selected_index(&mut self, index: Option<usize>) {
        self.selected_index = index;
        self.is_dirty = true;
    }
}
