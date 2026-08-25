use crate::prelude::*;

/// The layout mode.
#[derive(PartialEq, Clone, Debug)]
pub enum TheSharedVLayoutMode {
    Top,
    Shared,
    Bottom,
}

struct TheSharedVSplitter {
    id: TheId,
    limiter: TheSizeLimiter,
    dim: TheDim,
    dragging: bool,
    is_dirty: bool,
}

impl TheWidget for TheSharedVSplitter {
    fn new(id: TheId) -> Self
    where
        Self: Sized,
    {
        Self {
            id,
            limiter: TheSizeLimiter::new(),
            dim: TheDim::zero(),
            dragging: false,
            is_dirty: true,
        }
    }

    fn id(&self) -> &TheId {
        &self.id
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

    fn on_event(&mut self, event: &TheEvent, ctx: &mut TheContext) -> bool {
        match event {
            TheEvent::MouseDown(coord) => {
                self.dragging = true;
                ctx.ui.set_focus(self.id());
                ctx.ui
                    .send_widget_value_changed(self.id(), TheValue::Int(self.dim.y + coord.y));
                true
            }
            TheEvent::MouseDragged(coord) if self.dragging => {
                ctx.ui
                    .send_widget_value_changed(self.id(), TheValue::Int(self.dim.y + coord.y));
                true
            }
            TheEvent::MouseUp(coord) if self.dragging => {
                self.dragging = false;
                ctx.ui
                    .send_widget_value_changed(self.id(), TheValue::Int(self.dim.y + coord.y));
                ctx.ui.clear_focus();
                true
            }
            TheEvent::Hover(_) => {
                if !self.id().equals(&ctx.ui.hover) {
                    ctx.ui.set_hover(self.id());
                    self.is_dirty = true;
                }
                true
            }
            TheEvent::LostHover(_) => {
                self.is_dirty = true;
                true
            }
            _ => false,
        }
    }

    fn cursor_icon(&self) -> Option<TheCursorIcon> {
        Some(TheCursorIcon::RowResize)
    }

    fn supports_hover(&mut self) -> bool {
        true
    }

    fn needs_redraw(&mut self) -> bool {
        self.is_dirty
    }

    fn set_needs_redraw(&mut self, redraw: bool) {
        self.is_dirty = redraw;
    }

    fn draw(
        &mut self,
        buffer: &mut TheRGBABuffer,
        style: &mut Box<dyn TheStyle>,
        ctx: &mut TheContext,
    ) {
        if self.dim.is_valid() {
            let stride = buffer.stride();
            let rect = (
                self.dim.buffer_x.max(0) as usize,
                (self.dim.buffer_y + self.dim.height / 2).max(0) as usize,
                self.dim.width.max(0) as usize,
                1,
            );
            ctx.draw.rect(
                buffer.pixels_mut(),
                &rect,
                stride,
                style.theme().color(DividerStart),
            );
        }
        self.is_dirty = false;
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub struct TheSharedVLayout {
    id: TheId,
    limiter: TheSizeLimiter,

    mode: TheSharedVLayoutMode,
    dim: TheDim,

    margin: Vec4<i32>,
    padding: i32,

    canvas: Vec<TheCanvas>,
    widgets: Vec<Box<dyn TheWidget>>,

    background: Option<TheThemeColors>,
    ratio: f32,
    split_height: i32,
    is_dirty: bool,
}

impl TheLayout for TheSharedVLayout {
    fn new(id: TheId) -> Self
    where
        Self: Sized,
    {
        Self {
            id,
            limiter: TheSizeLimiter::new(),
            mode: TheSharedVLayoutMode::Top,

            dim: TheDim::zero(),

            canvas: vec![],
            widgets: vec![Box::new(TheSharedVSplitter::new(TheId::named(
                "Shared V Splitter",
            )))],

            margin: Vec4::new(10, 10, 10, 10),
            padding: 5,

            background: Some(DefaultWidgetBackground),
            ratio: 0.5,
            split_height: 0,
            is_dirty: true,
        }
    }

    fn id(&self) -> &TheId {
        &self.id
    }

    fn set_margin(&mut self, margin: Vec4<i32>) {
        self.margin = margin;
        self.is_dirty = true;
    }

    fn set_padding(&mut self, padding: i32) {
        self.padding = padding;
        self.is_dirty = true;
    }

    fn set_background_color(&mut self, color: Option<TheThemeColors>) {
        self.background = color;
        self.is_dirty = true;
    }

    fn widgets(&mut self) -> &mut Vec<Box<dyn TheWidget>> {
        &mut self.widgets
    }

    fn needs_redraw(&mut self) -> bool {
        if self.is_dirty || self.canvas.len() < 2 {
            return self.is_dirty;
        }

        let splitter_dirty = self.widgets.iter_mut().any(|widget| widget.needs_redraw());

        match self.mode {
            TheSharedVLayoutMode::Top => self.canvas[0].needs_redraw() || splitter_dirty,
            TheSharedVLayoutMode::Bottom => self.canvas[1].needs_redraw() || splitter_dirty,
            TheSharedVLayoutMode::Shared => {
                self.canvas[0].needs_redraw() || self.canvas[1].needs_redraw() || splitter_dirty
            }
        }
    }

    fn get_layout_at_coord(&mut self, coord: Vec2<i32>) -> Option<TheId> {
        if !self.dim.contains(coord) || self.canvas.len() < 2 {
            return None;
        }

        if self.mode == TheSharedVLayoutMode::Top {
            return self.canvas[0].get_layout_at_coord(coord);
        }
        if self.mode == TheSharedVLayoutMode::Bottom {
            return self.canvas[1].get_layout_at_coord(coord);
        }

        for c in &mut self.canvas {
            if let Some(layout_id) = c.get_layout_at_coord(coord) {
                return Some(layout_id);
            }
        }
        None
    }

    fn get_widget_at_coord(&mut self, coord: Vec2<i32>) -> Option<&mut Box<dyn TheWidget>> {
        if self.canvas.len() < 2 {
            return None;
        }

        if let Some(widget) = self
            .widgets
            .iter_mut()
            .find(|widget| widget.dim().contains(coord))
        {
            return Some(widget);
        }

        if self.mode == TheSharedVLayoutMode::Top {
            return self.canvas[0].get_widget_at_coord(coord);
        }
        if self.mode == TheSharedVLayoutMode::Bottom {
            return self.canvas[1].get_widget_at_coord(coord);
        }

        for c in self.canvas.iter_mut() {
            if let Some(w) = c.get_widget_at_coord(coord) {
                return Some(w);
            }
        }
        None
    }

    fn get_widget(
        &mut self,
        name: Option<&String>,
        uuid: Option<&Uuid>,
    ) -> Option<&mut Box<dyn TheWidget>> {
        if self.canvas.len() < 2 {
            return None;
        }

        if let Some(widget) = self
            .widgets
            .iter_mut()
            .find(|widget| widget.id().matches(name, uuid))
        {
            return Some(widget);
        }

        for c in self.canvas.iter_mut() {
            if let Some(w) = c.get_widget(name, uuid) {
                return Some(w);
            }
        }
        None
    }

    fn get_layout(
        &mut self,
        name: Option<&String>,
        uuid: Option<&Uuid>,
    ) -> Option<&mut Box<dyn TheLayout>> {
        if self.canvas.len() < 2 {
            return None;
        }

        for c in self.canvas.iter_mut() {
            if let Some(w) = c.get_layout(name, uuid) {
                return Some(w);
            }
        }
        None
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
            self.is_dirty = true;

            if self.canvas.len() < 2 {
                return;
            }

            if self.mode == TheSharedVLayoutMode::Top {
                self.widgets[0].set_dim(TheDim::zero(), ctx);
                self.canvas[0].set_dim(dim, ctx);
            } else if self.mode == TheSharedVLayoutMode::Bottom {
                self.widgets[0].set_dim(TheDim::zero(), ctx);
                self.canvas[1].set_dim(dim, ctx);
            } else {
                let available = (dim.height - 1).max(0);
                let desired = (dim.height as f32 * self.ratio) as i32;
                let top_min = self.canvas[0].limiter.get_min_height().clamp(0, available);
                let bottom_min = self.canvas[1].limiter.get_min_height().clamp(0, available);
                let top_height = if top_min + bottom_min <= available {
                    desired.clamp(top_min, available - bottom_min)
                } else if top_min + bottom_min > 0 {
                    // The host itself is smaller than the requested combined
                    // minimum. Divide the available area proportionally while
                    // still guaranteeing non-negative child dimensions.
                    ((available as i64 * top_min as i64) / (top_min + bottom_min) as i64) as i32
                } else {
                    desired.clamp(0, available)
                };
                let bottom_height = available - top_height;
                self.split_height = top_height;
                self.canvas[0].set_dim(TheDim::new(dim.x, dim.y, dim.width, top_height), ctx);
                self.canvas[1].set_dim(
                    TheDim::new(dim.x, dim.y + top_height + 1, dim.width, bottom_height),
                    ctx,
                );
                let mut splitter_dim = TheDim::new(dim.x, dim.y + top_height - 3, dim.width, 7);
                splitter_dim.set_buffer_offset(dim.buffer_x, dim.buffer_y + top_height - 3);
                self.widgets[0].set_dim(splitter_dim, ctx);
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
        if self.canvas.len() < 2 {
            return;
        }

        if let Some(background) = self.background {
            let stride = buffer.stride();

            ctx.draw.rect(
                buffer.pixels_mut(),
                &self.dim.to_buffer_utuple(),
                stride,
                style.theme().color(background),
            );
        }

        if self.mode == TheSharedVLayoutMode::Top {
            self.canvas[0].draw(style, ctx);

            buffer.copy_into(
                self.dim.buffer_x,
                self.dim.buffer_y,
                self.canvas[0].buffer(),
            );
        } else if self.mode == TheSharedVLayoutMode::Bottom {
            self.canvas[1].draw(style, ctx);
            buffer.copy_into(
                self.dim.buffer_x,
                self.dim.buffer_y,
                self.canvas[1].buffer(),
            );
        } else {
            self.canvas[0].draw(style, ctx);

            buffer.copy_into(
                self.dim.buffer_x,
                self.dim.buffer_y,
                self.canvas[0].buffer(),
            );

            self.canvas[1].draw(style, ctx);
            buffer.copy_into(
                self.dim.buffer_x,
                self.dim.buffer_y + self.split_height + 1,
                self.canvas[1].buffer(),
            );

            for widget in &mut self.widgets {
                widget.draw(buffer, style, ctx);
            }
        }
        self.is_dirty = false;
    }

    fn as_sharedvlayout(&mut self) -> Option<&mut dyn TheSharedVLayoutTrait> {
        Some(self)
    }
}

/// TheHLayout specific functions.
pub trait TheSharedVLayoutTrait: TheLayout {
    /// Add a canvas.
    fn add_canvas(&mut self, canvas: TheCanvas);
    /// Get a canvas.
    fn get_canvas_mut(&mut self, index: usize) -> Option<&mut TheCanvas>;
    /// Get the mode
    fn mode(&mut self) -> TheSharedVLayoutMode;
    /// Set the layout mode.
    fn set_mode(&mut self, mode: TheSharedVLayoutMode);
    /// Get the layout mode.
    fn get_mode(&self) -> TheSharedVLayoutMode;
    /// Get the current shared ratio.
    fn get_shared_ratio(&self) -> f32;
    /// Move the shared divider to an absolute screen y-coordinate.
    fn set_split_position(&mut self, screen_y: i32);
    // Se the shared ratio. Default is 0.5.
    fn set_shared_ratio(&mut self, ratio: f32);
}

impl TheSharedVLayoutTrait for TheSharedVLayout {
    fn add_canvas(&mut self, canvas: TheCanvas) {
        self.canvas.push(canvas);
        self.is_dirty = true;
    }
    fn get_canvas_mut(&mut self, index: usize) -> Option<&mut TheCanvas> {
        if index < self.canvas.len() {
            return Some(&mut self.canvas[index]);
        }
        None
    }
    fn mode(&mut self) -> TheSharedVLayoutMode {
        self.mode.clone()
    }
    fn set_mode(&mut self, mode: TheSharedVLayoutMode) {
        if self.mode != mode {
            self.mode = mode;
            self.is_dirty = true;
        }
    }
    fn get_mode(&self) -> TheSharedVLayoutMode {
        self.mode.clone()
    }
    fn get_shared_ratio(&self) -> f32 {
        self.ratio
    }
    fn set_split_position(&mut self, screen_y: i32) {
        if self.mode != TheSharedVLayoutMode::Shared
            || self.dim.height <= 1
            || self.canvas.len() < 2
        {
            return;
        }

        let available = self.dim.height - 1;
        let desired = (screen_y - self.dim.y).clamp(0, available);
        let top_min = self.canvas[0].limiter.get_min_height().clamp(0, available);
        let bottom_min = self.canvas[1].limiter.get_min_height().clamp(0, available);
        let top_height = if top_min + bottom_min <= available {
            desired.clamp(top_min, available - bottom_min)
        } else if top_min + bottom_min > 0 {
            ((available as i64 * top_min as i64) / (top_min + bottom_min) as i64) as i32
        } else {
            desired
        };
        self.set_shared_ratio(top_height as f32 / self.dim.height as f32);
    }
    fn set_shared_ratio(&mut self, ratio: f32) {
        let ratio = ratio.clamp(0.0, 1.0);
        if (self.ratio - ratio).abs() > f32::EPSILON {
            self.ratio = ratio;
            self.is_dirty = true;
        }
    }
}
