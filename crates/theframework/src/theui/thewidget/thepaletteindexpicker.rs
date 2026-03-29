use crate::prelude::*;
use crate::thecontext::TheCursorIcon;

pub struct ThePaletteIndexPicker {
    id: TheId,
    limiter: TheSizeLimiter,
    status: Option<String>,

    is_disabled: bool,
    state: TheWidgetState,

    palette: ThePalette,
    selected: i32,
    original: i32,

    dim: TheDim,
    is_dirty: bool,

    safety_offset: Vec2<i32>,
    embedded: bool,
    parent_id: Option<TheId>,
    overlay_offset: Vec2<i32>,
}

impl TheWidget for ThePaletteIndexPicker {
    fn new(id: TheId) -> Self
    where
        Self: Sized,
    {
        let mut limiter = TheSizeLimiter::new();
        limiter.set_max_width(142);
        limiter.set_max_height(20);

        Self {
            id,
            limiter,
            status: None,
            is_disabled: false,
            state: TheWidgetState::None,
            palette: ThePalette::default(),
            selected: 0,
            original: 0,
            dim: TheDim::zero(),
            is_dirty: true,
            safety_offset: Vec2::zero(),
            embedded: false,
            parent_id: None,
            overlay_offset: Vec2::zero(),
        }
    }

    fn cursor_icon(&self) -> Option<TheCursorIcon> {
        Some(TheCursorIcon::Hand)
    }

    fn set_cursor_icon(&mut self, _icon: Option<TheCursorIcon>) {}

    fn id(&self) -> &TheId {
        &self.id
    }

    fn value(&self) -> TheValue {
        TheValue::Int(self.selected)
    }

    fn set_value(&mut self, value: TheValue) {
        if let Some(index) = value.to_i32() {
            self.selected = index.clamp(0, 255);
            self.is_dirty = true;
        }
    }

    fn status_text(&self) -> Option<String> {
        self.status.clone()
    }

    fn set_status_text(&mut self, text: &str) {
        self.status = Some(text.to_string());
    }

    fn disabled(&self) -> bool {
        self.is_disabled
    }

    fn set_disabled(&mut self, disabled: bool) {
        if disabled != self.is_disabled {
            self.is_disabled = disabled;
            self.is_dirty = true;
        }
    }

    fn set_embedded(&mut self, embedded: bool) {
        self.embedded = embedded;
    }

    fn set_parent_id(&mut self, parent_id: TheId) {
        self.parent_id = Some(parent_id);
    }

    fn parent_id(&self) -> Option<&TheId> {
        self.parent_id.as_ref()
    }

    fn on_event(&mut self, event: &TheEvent, ctx: &mut TheContext) -> bool {
        let mut redraw = false;
        if self.is_disabled {
            return false;
        }
        match event {
            TheEvent::MouseDown(_coord) => {
                self.is_dirty = true;
                if self.state != TheWidgetState::Clicked {
                    self.state = TheWidgetState::Clicked;
                    ctx.ui.send_widget_state_changed(self.id(), self.state);
                    ctx.ui.set_focus(self.id());
                    if let Some(parent_id) = &self.parent_id {
                        ctx.ui.set_overlay(parent_id);
                    } else {
                        ctx.ui.set_overlay(self.id());
                    }
                    self.original = self.selected;
                }
                redraw = true;
            }
            TheEvent::MouseDragged(coord) => {
                if self.state == TheWidgetState::Clicked
                    && let Some(index) = self.index_at(*coord)
                {
                    if index != self.selected {
                        self.selected = index;
                        self.is_dirty = true;
                        redraw = true;
                    }
                }
            }
            TheEvent::MouseUp(coord) => {
                self.is_dirty = true;
                if self.state == TheWidgetState::Clicked {
                    if let Some(index) = self.index_at(*coord) {
                        self.selected = index;
                    }
                    self.state = TheWidgetState::None;
                    ctx.ui.send_widget_state_changed(self.id(), self.state);
                    ctx.ui.clear_overlay();
                    if self.selected != self.original {
                        ctx.ui
                            .send_widget_value_changed(self.id(), TheValue::Int(self.selected));
                        ctx.ui.send(TheEvent::PaletteIndexChanged(
                            self.id().clone(),
                            self.selected as u16,
                        ));
                    }
                }
                redraw = true;
            }
            TheEvent::Hover(_) => {
                if self.state != TheWidgetState::Clicked && !self.id().equals(&ctx.ui.hover) {
                    self.is_dirty = true;
                    ctx.ui.set_hover(self.id());
                    redraw = true;
                }
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
        let utuple = self.dim.to_buffer_utuple();

        let mut icon_name = if self.state == TheWidgetState::Clicked && !self.embedded {
            "dark_dropdown_clicked".to_string()
        } else {
            "dark_dropdown_normal".to_string()
        };
        if !self.is_disabled && !self.embedded {
            if self.state != TheWidgetState::Clicked && self.id().equals(&ctx.ui.hover) {
                icon_name = "dark_dropdown_hover".to_string();
            }
            if self.state != TheWidgetState::Clicked && self.id().equals(&ctx.ui.focus) {
                icon_name = "dark_dropdown_focus".to_string();
            }
        }

        if let Some(icon) = ctx.ui.icon(&icon_name) {
            let off = if icon.dim().width == 140 { 1 } else { 0 };
            let r = (
                utuple.0 + off,
                utuple.1 + off,
                icon.dim().width as usize,
                icon.dim().height as usize,
            );
            ctx.draw
                .blend_slice(buffer.pixels_mut(), icon.pixels(), &r, stride);
        }

        if let Some(icon) = ctx.ui.icon("dark_dropdown_marker") {
            let r = (
                utuple.0 + 129,
                utuple.1 + 7,
                icon.dim().width as usize,
                icon.dim().height as usize,
            );
            ctx.draw
                .blend_slice(buffer.pixels_mut(), icon.pixels(), &r, stride);
        }

        let swatch_rect = (utuple.0 + 8, utuple.1 + 4, 16, 12);
        let color = self
            .palette
            .colors
            .get(self.selected as usize)
            .and_then(|c| c.clone())
            .unwrap_or(TheColor::from([0_u8, 0, 0, 255]));
        ctx.draw.rect(
            buffer.pixels_mut(),
            &swatch_rect,
            stride,
            &color.to_u8_array(),
        );
        ctx.draw
            .rect_outline(buffer.pixels_mut(), &swatch_rect, stride, &BLACK);

        ctx.draw.text_rect_blend(
            buffer.pixels_mut(),
            &(utuple.0 + 30, utuple.1, 96, utuple.3),
            stride,
            &format!("#{}", self.selected),
            TheFontSettings {
                size: 12.5,
                ..Default::default()
            },
            style.theme().color(SectionbarNormalTextColor),
            TheHorizontalAlign::Left,
            TheVerticalAlign::Center,
        );

        self.is_dirty = false;
    }

    fn draw_overlay(
        &mut self,
        style: &mut Box<dyn TheStyle>,
        ctx: &mut TheContext,
    ) -> TheRGBABuffer {
        let count = self.visible_color_count();
        let columns = if count >= 144 {
            16
        } else if count >= 64 {
            12
        } else {
            8
        }
        .min(count);
        let rows = count.div_ceil(columns);
        let cell = 18usize;
        let padding = 10usize;
        let spacing = 2usize;
        let width = padding * 2 + columns * cell + (columns.saturating_sub(1)) * spacing;
        let height = padding * 2 + rows * cell + (rows.saturating_sub(1)) * spacing;

        let mut dim = TheDim::new(self.dim.x, self.dim.y + 20, width as i32, height as i32);
        dim.buffer_x = self.dim.x;
        dim.buffer_y = self.dim.y + 20;
        self.overlay_offset = Vec2::new(self.dim.x, self.dim.y + 20);
        self.safety_offset = Vec2::zero();

        if dim.x + width as i32 > ctx.width as i32 {
            self.safety_offset.x = dim.x + width as i32 - ctx.width as i32 + 5;
            dim.x -= self.safety_offset.x;
        }
        if dim.y + height as i32 > ctx.height as i32 {
            self.safety_offset.y = dim.y + height as i32 - ctx.height as i32 + 5;
            dim.y -= self.safety_offset.y;
        }

        let mut buffer = TheRGBABuffer::new(dim);
        ctx.draw.rect(
            buffer.pixels_mut(),
            &(0, 0, width, height),
            width,
            style.theme().color(MenubarPopupBackground),
        );
        ctx.draw.rect_outline(
            buffer.pixels_mut(),
            &(0, 0, width, height),
            width,
            style.theme().color(MenubarPopupBorder),
        );

        for index in 0..count {
            let row = index / columns;
            let col = index % columns;
            let x = padding + col * (cell + spacing);
            let y = padding + row * (cell + spacing);
            let rect = (x, y, cell, cell);
            if index as i32 == self.selected {
                ctx.draw.rect(
                    buffer.pixels_mut(),
                    &rect,
                    width,
                    style.theme().color(DefaultSelection),
                );
            }
            let inner = (x + 2, y + 2, cell - 4, cell - 4);
            let color = self
                .palette
                .colors
                .get(index)
                .and_then(|c| c.clone())
                .unwrap_or(TheColor::from([0_u8, 0, 0, 255]));
            ctx.draw
                .rect(buffer.pixels_mut(), &inner, width, &color.to_u8_array());
            ctx.draw
                .rect_outline(buffer.pixels_mut(), &inner, width, &BLACK);
        }

        buffer
    }

    fn as_palette_index_picker(&mut self) -> Option<&mut dyn ThePaletteIndexPickerTrait> {
        Some(self)
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl ThePaletteIndexPicker {
    fn visible_color_count(&self) -> usize {
        self.palette
            .colors
            .iter()
            .rposition(|c| c.is_some())
            .map(|i| i + 1)
            .unwrap_or(1)
    }

    fn index_at(&self, coord: Vec2<i32>) -> Option<i32> {
        let count = self.visible_color_count();
        let columns = if count >= 144 {
            16
        } else if count >= 64 {
            12
        } else {
            8
        }
        .min(count);
        let cell = 18i32;
        let padding = 10i32;
        let spacing = 2i32;
        let local = Vec2::new(coord.x, coord.y - 20);
        if local.x < padding || local.y < padding {
            return None;
        }
        let x_rel = local.x - padding;
        let y_rel = local.y - padding;
        let col = x_rel / (cell + spacing);
        let row = y_rel / (cell + spacing);
        if col < 0 || row < 0 {
            return None;
        }
        let x_in = x_rel % (cell + spacing);
        let y_in = y_rel % (cell + spacing);
        if x_in >= cell || y_in >= cell {
            return None;
        }
        let index = row as usize * columns + col as usize;
        (index < count).then_some(index as i32)
    }
}

pub trait ThePaletteIndexPickerTrait: TheWidget {
    fn set_palette(&mut self, palette: ThePalette);
    fn set_selected_index(&mut self, index: i32);
    fn selected_index(&self) -> usize;
}

impl ThePaletteIndexPickerTrait for ThePaletteIndexPicker {
    fn set_palette(&mut self, palette: ThePalette) {
        self.palette = palette;
        self.is_dirty = true;
    }

    fn set_selected_index(&mut self, index: i32) {
        self.selected = index.clamp(0, 255);
        self.is_dirty = true;
    }

    fn selected_index(&self) -> usize {
        self.selected.max(0) as usize
    }
}
