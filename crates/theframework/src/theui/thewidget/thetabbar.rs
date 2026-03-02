use crate::prelude::*;

pub struct TheTabbar {
    id: TheId,
    limiter: TheSizeLimiter,

    state: TheWidgetState,

    tabs: Vec<String>,
    selected: i32,
    original: i32,

    selected_index: Option<i32>,
    hover_index: Option<i32>,

    dim: TheDim,
    is_dirty: bool,
}

impl TheWidget for TheTabbar {
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

            tabs: vec![],
            selected: 0,
            original: 0,

            selected_index: Some(0),
            hover_index: None,

            dim: TheDim::zero(),
            is_dirty: false,
        }
    }

    fn id(&self) -> &TheId {
        &self.id
    }

    fn on_event(&mut self, event: &TheEvent, ctx: &mut TheContext) -> bool {
        let mut redraw = false;
        // println!("event ({}): {:?}", self.widget_id.name, event);
        match event {
            TheEvent::MouseDown(coord) => {
                self.is_dirty = true;
                if self.state != TheWidgetState::Selected {
                    self.state = TheWidgetState::Selected;
                    ctx.ui.send_widget_state_changed(self.id(), self.state);
                    ctx.ui.set_focus(self.id());
                    self.original = self.selected;
                    redraw = true;
                }
                let index = coord.x / 142;
                if index >= 0 && index < self.tabs.len() as i32 {
                    if Some(index) != self.selected_index {
                        self.selected_index = Some(index);
                        ctx.ui
                            .send(TheEvent::IndexChanged(self.id.clone(), index as usize));
                        redraw = true;
                        self.is_dirty = true;
                    }
                } else if self.selected_index.is_some() {
                    self.selected_index = None;
                    redraw = true;
                    self.is_dirty = true;
                }
            }
            TheEvent::Hover(coord) => {
                if !self.id().equals(&ctx.ui.hover) {
                    ctx.ui.set_hover(self.id());
                    redraw = true;
                    self.is_dirty = true;
                }
                let index = coord.x / 142;
                if index >= 0 && index < self.tabs.len() as i32 {
                    if Some(index) != self.hover_index {
                        self.hover_index = Some(index);
                        redraw = true;
                        self.is_dirty = true;
                    }
                } else if self.hover_index.is_some() {
                    self.hover_index = None;
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
        let buffer_height = buffer.dim().height.max(0) as usize;

        let utuple: (usize, usize, usize, usize) = self.dim.to_buffer_utuple();

        ctx.draw.rect(
            buffer.pixels_mut(),
            &utuple,
            stride,
            style.theme().color(TabbarBackground),
        );

        let mut x = 0;

        for (index, text) in self.tabs.iter().enumerate() {
            let mut icon_name = if Some(index as i32) == self.selected_index {
                "dark_tabbar_selected".to_string()
            } else {
                "dark_tabbar_normal".to_string()
            };

            if Some(index as i32) == self.hover_index && icon_name != "dark_tabbar_selected" {
                icon_name = "dark_tabbar_hover".to_string()
            }

            if let Some(icon) = ctx.ui.icon(&icon_name) {
                let icon_w = icon.dim().width as usize;
                let icon_h = icon.dim().height as usize;
                let draw_x = utuple.0 + x;
                let draw_y = utuple.1;

                // Guard against overflow when many tabs exceed available width/height.
                if draw_x >= stride
                    || draw_y >= buffer_height
                    || draw_x + icon_w > stride
                    || draw_y + icon_h > buffer_height
                {
                    break;
                }

                let r = (draw_x, draw_y, icon_w, icon_h);
                ctx.draw
                    .copy_slice(buffer.pixels_mut(), icon.pixels(), &r, stride);

                ctx.draw.text_rect_blend(
                    buffer.pixels_mut(),
                    &r,
                    stride,
                    text.as_str(),
                    TheFontSettings {
                        size: 12.5,
                        ..Default::default()
                    },
                    style.theme().color(TabbarText),
                    TheHorizontalAlign::Center,
                    TheVerticalAlign::Center,
                );

                x += icon_w;
            } else {
                // Fallback when tabbar skin icons are unavailable.
                let tab_w = 142usize;
                let tab_h = utuple.3.saturating_sub(1);
                let draw_x = utuple.0 + x;
                let draw_y = utuple.1;
                if draw_x >= stride
                    || draw_y >= buffer_height
                    || draw_x + tab_w > stride
                    || draw_y + tab_h > buffer_height
                {
                    break;
                }

                let selected = Some(index as i32) == self.selected_index;
                let bg = if selected {
                    style.theme().color(TabbarConnector)
                } else {
                    style.theme().color(TabbarBackground)
                };
                let r = (draw_x, draw_y, tab_w, tab_h);
                ctx.draw.rect(buffer.pixels_mut(), &r, stride, bg);
                ctx.draw.text_rect_blend(
                    buffer.pixels_mut(),
                    &r,
                    stride,
                    text.as_str(),
                    TheFontSettings {
                        size: 12.5,
                        ..Default::default()
                    },
                    style.theme().color(TabbarText),
                    TheHorizontalAlign::Center,
                    TheVerticalAlign::Center,
                );

                x += tab_w;
            }

            if index < self.tabs.len() - 1 {
                // Connector

                let r = (utuple.0 + x, utuple.1 + utuple.3 - 1, 2, 1);
                ctx.draw.rect(
                    buffer.pixels_mut(),
                    &r,
                    stride,
                    style.theme().color(TabbarConnector),
                );

                x += 2;
            }
        }

        self.is_dirty = false;
    }

    fn as_tabbar(&mut self) -> Option<&mut dyn TheTabbarTrait> {
        Some(self)
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub trait TheTabbarTrait {
    fn clear(&mut self);
    fn add_tab(&mut self, name: String);
    fn selection_index(&self) -> Option<i32>;
    fn selection(&self) -> Option<String>;
    fn set_selection(&mut self, name: String);
    fn set_selection_index(&mut self, index: usize);
}

impl TheTabbarTrait for TheTabbar {
    fn clear(&mut self) {
        self.tabs.clear();
        self.is_dirty = true;
    }
    fn add_tab(&mut self, name: String) {
        self.tabs.push(name);
    }
    fn selection_index(&self) -> Option<i32> {
        self.selected_index
    }
    fn selection(&self) -> Option<String> {
        if let Some(index) = self.selected_index {
            if index < self.tabs.len() as i32 {
                return Some(self.tabs[index as usize].clone());
            }
        }
        None
    }
    fn set_selection(&mut self, name: String) {
        self.is_dirty = true;
        for (index, text) in self.tabs.iter().enumerate() {
            if name == *text {
                self.selected_index = Some(index as i32);
                return;
            }
        }
        self.selected_index = None;
    }
    fn set_selection_index(&mut self, index: usize) {
        self.is_dirty = true;
        self.selected_index = Some(index as i32);
    }
}
