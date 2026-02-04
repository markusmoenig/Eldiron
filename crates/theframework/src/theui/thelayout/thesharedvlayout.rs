use crate::prelude::*;

/// The layout mode.
#[derive(PartialEq, Clone, Debug)]
pub enum TheSharedVLayoutMode {
    Top,
    Shared,
    Bottom,
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
            widgets: vec![],

            margin: Vec4::new(10, 10, 10, 10),
            padding: 5,

            background: Some(DefaultWidgetBackground),
            ratio: 0.5,
        }
    }

    fn id(&self) -> &TheId {
        &self.id
    }

    fn set_margin(&mut self, margin: Vec4<i32>) {
        self.margin = margin;
    }

    fn set_padding(&mut self, padding: i32) {
        self.padding = padding;
    }

    fn set_background_color(&mut self, color: Option<TheThemeColors>) {
        self.background = color;
    }

    fn widgets(&mut self) -> &mut Vec<Box<dyn TheWidget>> {
        &mut self.widgets
    }

    fn needs_redraw(&mut self) -> bool {
        for canvas in &mut self.canvas {
            if canvas.needs_redraw() {
                return true;
            }
        }

        false
    }

    fn get_layout_at_coord(&mut self, coord: Vec2<i32>) -> Option<TheId> {
        if self.dim.contains(coord) {
            for c in &mut self.canvas {
                if let Some(layout_id) = c.get_layout_at_coord(coord) {
                    return Some(layout_id);
                }
            }
        }
        None
    }

    fn get_widget_at_coord(&mut self, coord: Vec2<i32>) -> Option<&mut Box<dyn TheWidget>> {
        if self.canvas.len() < 2 {
            return None;
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

            if self.canvas.len() < 2 {
                return;
            }

            if self.mode == TheSharedVLayoutMode::Top {
                self.canvas[0].set_dim(dim, ctx);
            } else if self.mode == TheSharedVLayoutMode::Bottom {
                self.canvas[1].set_dim(dim, ctx);
            } else {
                self.canvas[0].set_dim(
                    TheDim::new(
                        dim.x,
                        dim.y,
                        dim.width,
                        (dim.height as f32 * self.ratio) as i32,
                    ),
                    ctx,
                );
                self.canvas[1].set_dim(
                    TheDim::new(
                        dim.x,
                        dim.y + (dim.height as f32 * self.ratio) as i32 + 1,
                        dim.width,
                        (dim.height - (dim.height as f32 * self.ratio) as i32) - 1,
                    ),
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
                self.dim.buffer_y + (self.dim.height as f32 * self.ratio) as i32 + 1,
                self.canvas[1].buffer(),
            );
        }
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
    // Se the shared ratio. Default is 0.5.
    fn set_shared_ratio(&mut self, ratio: f32);
}

impl TheSharedVLayoutTrait for TheSharedVLayout {
    fn add_canvas(&mut self, canvas: TheCanvas) {
        self.canvas.push(canvas);
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
        self.mode = mode;
    }
    fn get_mode(&self) -> TheSharedVLayoutMode {
        self.mode.clone()
    }
    fn set_shared_ratio(&mut self, ratio: f32) {
        self.ratio = ratio;
    }
}
