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

        let bar_rect = ThePixelRect::new(
            self.dim.buffer_x,
            self.dim.buffer_y,
            self.dim.width,
            self.dim.height,
        );
        let bar_paint = style.theme().paint(TabbarChrome, bar_rect);
        let connector = *style.theme().color(TabbarConnector);
        let text_color = *style.theme().color(TabbarText);
        let width = buffer.dim().width.max(0) as usize;
        let height = buffer.dim().height.max(0) as usize;
        let buffer_bounds = ThePixelRect::new(0, 0, width as i32, height as i32);
        let mut tab_rects = Vec::with_capacity(self.tabs.len());

        if let Ok(mut surface) = TheSurfaceMut::new(buffer.pixels_mut(), width, height) {
            surface.set_clip(bar_rect);
            ctx.painter.fill_rect(&mut surface, bar_rect, &bar_paint);

            let mut x = bar_rect.x;
            for index in 0..self.tabs.len() {
                let tab_rect =
                    ThePixelRect::new(x, bar_rect.y, 142, bar_rect.height.saturating_sub(1));
                let role = if Some(index as i32) == self.selected_index {
                    TabSelectedChrome
                } else if Some(index as i32) == self.hover_index {
                    TabHoverChrome
                } else {
                    TabNormalChrome
                };
                let paint = style.theme().paint(role, tab_rect);
                ctx.painter.fill_rect(&mut surface, tab_rect, &paint);
                tab_rects.push(tab_rect);
                x = x.saturating_add(142);

                if index + 1 < self.tabs.len() {
                    surface.fill_rect(
                        ThePixelRect::new(
                            x,
                            bar_rect.y.saturating_add(bar_rect.height.saturating_sub(1)),
                            2,
                            1,
                        ),
                        connector,
                    );
                    x = x.saturating_add(2);
                }
            }
        }

        let stride = buffer.stride();
        for (text, tab_rect) in self.tabs.iter().zip(tab_rects) {
            let clipped = tab_rect.intersection(buffer_bounds).intersection(bar_rect);
            if clipped.is_empty() {
                continue;
            }
            let text_rect = (
                clipped.x as usize,
                clipped.y as usize,
                clipped.width as usize,
                clipped.height as usize,
            );
            ctx.draw.text_rect_blend(
                buffer.pixels_mut(),
                &text_rect,
                stride,
                text,
                TheFontSettings {
                    size: 12.5,
                    ..Default::default()
                },
                &text_color,
                TheHorizontalAlign::Center,
                TheVerticalAlign::Center,
            );
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
