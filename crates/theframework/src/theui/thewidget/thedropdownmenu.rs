use crate::prelude::*;
use crate::thecontext::TheCursorIcon;

pub struct TheDropdownMenu {
    id: TheId,
    limiter: TheSizeLimiter,
    status: Option<String>,

    is_disabled: bool,

    state: TheWidgetState,

    options: Vec<String>,
    selected: i32,
    original: i32,

    dim: TheDim,
    is_dirty: bool,

    safety_offset: Vec2<i32>,
    embedded: bool,

    parent_id: Option<TheId>,
    overlay_offset: Vec2<i32>,
}

impl TheWidget for TheDropdownMenu {
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

            options: vec![],
            selected: 0,
            original: 0,

            dim: TheDim::zero(),
            is_dirty: false,

            safety_offset: Vec2::new(0, 0),
            embedded: false,

            parent_id: None,
            overlay_offset: Vec2::zero(),
        }
    }

    fn cursor_icon(&self) -> Option<TheCursorIcon> {
        Some(TheCursorIcon::Hand)
    }

    fn set_cursor_icon(&mut self, _icon: Option<TheCursorIcon>) {
        // Store the cursor icon if needed for dynamic changes
    }

    fn id(&self) -> &TheId {
        &self.id
    }

    fn value(&self) -> TheValue {
        TheValue::Int(self.selected)
    }

    fn set_value(&mut self, value: TheValue) {
        if let Some(index) = value.to_i32() {
            if index >= 0 && index < self.options.len() as i32 {
                self.selected = index;
                self.is_dirty = true;
            }
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

                    // When embedded, set overlay to parent so parent's draw_overlay is called
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
                if !self.options.is_empty() {
                    let y: i32 = coord.y - 20 + self.safety_offset.y;
                    if y >= 0 {
                        let index = y / 20;
                        if index < self.options.len() as i32 && index != self.selected {
                            self.selected = index;
                        }
                    }
                    self.is_dirty = true;
                    redraw = true;
                }
            }
            TheEvent::MouseUp(_coord) => {
                self.is_dirty = true;
                if self.state == TheWidgetState::Clicked {
                    self.state = TheWidgetState::None;
                    ctx.ui.send_widget_state_changed(self.id(), self.state);
                    ctx.ui.clear_overlay();

                    if self.selected != self.original {
                        ctx.ui
                            .send_widget_value_changed(self.id(), TheValue::Int(self.selected));
                        ctx.ui.send(TheEvent::IndexChanged(
                            self.id().clone(),
                            self.selected as usize,
                        ));
                    }
                }
                redraw = true;
            }
            TheEvent::Hover(_coord) => {
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
        let mut shrinker = TheDimShrinker::zero();

        let utuple: (usize, usize, usize, usize) = self.dim.to_buffer_utuple();

        let mut icon_name = if self.state == TheWidgetState::Clicked && !self.embedded {
            "dark_dropdown_clicked".to_string()
        } else {
            "dark_dropdown_normal".to_string()
        };

        if !self.is_disabled && !self.embedded {
            if self.state != TheWidgetState::Clicked && self.id().equals(&ctx.ui.hover) {
                icon_name = "dark_dropdown_hover".to_string()
            }
            if self.state != TheWidgetState::Clicked && self.id().equals(&ctx.ui.focus) {
                icon_name = "dark_dropdown_focus".to_string()
            }
        }

        let text_color = if self.state == TheWidgetState::Selected {
            style.theme().color(SectionbarSelectedTextColor)
        } else {
            style.theme().color(SectionbarNormalTextColor)
        };

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

        shrinker.shrink_by(8, 0, 12, 0);

        if !self.options.is_empty() {
            ctx.draw.text_rect_blend(
                buffer.pixels_mut(),
                &self.dim.to_buffer_shrunk_utuple(&shrinker),
                stride,
                self.options[self.selected as usize].as_str(),
                TheFontSettings {
                    size: 12.5,
                    ..Default::default()
                },
                text_color,
                TheHorizontalAlign::Left,
                TheVerticalAlign::Center,
            );
        }

        self.is_dirty = false;
    }

    fn draw_overlay(
        &mut self,
        style: &mut Box<dyn TheStyle>,
        ctx: &mut TheContext,
    ) -> TheRGBABuffer {
        let len = self.options.len();
        let width = 142;

        #[allow(clippy::implicit_saturating_sub)]
        let height = 2 + len * 20 + (if len > 1 { len - 1 } else { 0 });

        let mut dim = TheDim::new(self.dim.x, self.dim.y + 20, width as i32, height as i32);
        dim.buffer_x = self.dim.x;
        dim.buffer_y = self.dim.y + 20;

        // Store the overlay offset (will be adjusted by parent if embedded)
        self.overlay_offset = Vec2::new(self.dim.x, self.dim.y + 20);

        self.safety_offset = Vec2::zero();

        // Safety check for width being larger than the window width
        if dim.x + width as i32 > ctx.width as i32 {
            self.safety_offset.x = dim.x + width as i32 - ctx.width as i32 + 5;
            dim.x -= self.safety_offset.x;
        }

        // Safety check for height being larger than the window height
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

        let x = 0;
        let mut y = 0;

        for i in 0..len {
            if i == self.selected as usize {
                ctx.draw.rect(
                    buffer.pixels_mut(),
                    &(x, y, width, 21),
                    width,
                    style.theme().color(DefaultSelection),
                );
            }

            ctx.draw.rect_outline(
                buffer.pixels_mut(),
                &(x, y, width, 21),
                width,
                style.theme().color(MenubarPopupBorder),
            );

            if !self.options.is_empty() {
                ctx.draw.text_rect_blend(
                    buffer.pixels_mut(),
                    &(x + 8, y, width - 8, 21),
                    width,
                    self.options[i].as_str(),
                    TheFontSettings {
                        size: 12.5,
                        ..Default::default()
                    },
                    &WHITE,
                    TheHorizontalAlign::Left,
                    TheVerticalAlign::Center,
                );
            }

            y += 21;
        }

        buffer
    }

    fn as_drop_down_menu(&mut self) -> Option<&mut dyn TheDropdownMenuTrait> {
        Some(self)
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub trait TheDropdownMenuTrait: TheWidget {
    fn add_option(&mut self, option: String);
    fn selected_text(&self) -> String;
    fn set_selected_index(&mut self, index: i32);
    fn selected_index(&self) -> usize;
}

impl TheDropdownMenuTrait for TheDropdownMenu {
    fn add_option(&mut self, option: String) {
        self.options.push(option);
    }
    fn selected_index(&self) -> usize {
        self.selected as usize
    }
    fn selected_text(&self) -> String {
        self.options[self.selected as usize].clone()
    }
    fn set_selected_index(&mut self, index: i32) {
        self.selected = index;
        self.is_dirty = true;
    }
}
