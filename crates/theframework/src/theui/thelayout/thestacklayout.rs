use crate::prelude::*;

pub struct TheStackLayout {
    id: TheId,
    dim: TheDim,
    limiter: TheSizeLimiter,

    canvas: Vec<TheCanvas>,

    widgets: Vec<Box<dyn TheWidget>>,
    layouts: Vec<Box<dyn TheLayout>>,
    index: usize,
}

impl TheLayout for TheStackLayout {
    fn new(id: TheId) -> Self
    where
        Self: Sized,
    {
        Self {
            id,
            dim: TheDim::zero(),
            limiter: TheSizeLimiter::new(),

            canvas: vec![],

            widgets: vec![],
            layouts: vec![],
            index: 0,
        }
    }

    fn id(&self) -> &TheId {
        &self.id
    }

    fn set_margin(&mut self, margin: Vec4<i32>) {
        if !self.layouts.is_empty() && self.index < self.layouts.len() {
            self.layouts[self.index].set_margin(margin);
        }
    }

    fn set_padding(&mut self, padding: i32) {
        if !self.layouts.is_empty() && self.index < self.layouts.len() {
            self.layouts[self.index].set_padding(padding);
        }
    }

    fn set_background_color(&mut self, color: Option<TheThemeColors>) {
        if !self.layouts.is_empty() && self.index < self.layouts.len() {
            self.layouts[self.index].set_background_color(color);
        }
    }

    fn needs_redraw(&mut self) -> bool {
        for canvas in &mut self.canvas {
            if canvas.needs_redraw() {
                return true;
            }
        }

        false
    }

    fn widgets(&mut self) -> &mut Vec<Box<dyn TheWidget>> {
        &mut self.widgets
    }

    fn get_layout_at_coord(&mut self, coord: Vec2<i32>) -> Option<TheId> {
        if self.dim.contains(coord) {
            if !self.canvas.is_empty() && self.index < self.canvas.len() {
                if let Some(layout_id) = self.canvas[self.index].get_layout_at_coord(coord) {
                    return Some(layout_id);
                }
            }
        }
        None
    }

    fn get_widget_at_coord(&mut self, coord: Vec2<i32>) -> Option<&mut Box<dyn TheWidget>> {
        if !self.canvas.is_empty() && self.index < self.canvas.len() {
            return self.canvas[self.index].get_widget_at_coord(coord);
        }
        None
    }

    fn get_layout(
        &mut self,
        name: Option<&String>,
        uuid: Option<&Uuid>,
    ) -> Option<&mut Box<dyn TheLayout>> {
        for canvas in &mut self.canvas {
            if let Some(layout) = canvas.get_layout(name, uuid) {
                return Some(layout);
            }
        }
        None
    }

    fn get_widget(
        &mut self,
        name: Option<&String>,
        uuid: Option<&Uuid>,
    ) -> Option<&mut Box<dyn TheWidget>> {
        for canvas in &mut self.canvas {
            if let Some(widget) = canvas.get_widget(name, uuid) {
                return Some(widget);
            }
        }
        // if !self.canvas.is_empty() && self.index < self.canvas.len() {
        //     return self.canvas[self.index].get_widget(name, uuid);
        // }
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
            // if !self.canvas.is_empty() && self.index < self.canvas.len() {
            //     self.canvas[self.index].set_dim(dim, ctx);
            // }

            // As all canvas have the same size anyway, there is no harm initializing them all.
            // This solves the problem  that when switching to a new canvas it is properly
            // configured and does not need a refresh.
            for c in &mut self.canvas {
                c.set_dim(dim, ctx);
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
        if !self.canvas.is_empty() && self.index < self.canvas.len() {
            self.canvas[self.index].draw(style, ctx);
            buffer.copy_into(
                self.dim.buffer_x,
                self.dim.buffer_y,
                self.canvas[self.index].buffer(),
            );
        }
    }

    /// Convert to the stack layout trait
    fn as_stack_layout(&mut self) -> Option<&mut dyn TheStackLayoutTrait> {
        Some(self)
    }
}

/// TheHLayout specific functions.
pub trait TheStackLayoutTrait: TheLayout {
    /// Add a canvas to the stack and returns the index.
    fn add_canvas(&mut self, canvas: TheCanvas) -> usize;

    /// Returns the index of the current layout.
    fn index(&self) -> usize;

    /// Set the index of the current layout.
    fn set_index(&mut self, index: usize);

    /// Get a mutable reference to the canvas at the given index.
    fn canvas_at_mut(&mut self, index: usize) -> Option<&mut TheCanvas>;
}

impl TheStackLayoutTrait for TheStackLayout {
    fn add_canvas(&mut self, canvas: TheCanvas) -> usize {
        let index = self.canvas.len();
        self.canvas.push(canvas);
        index
    }

    fn index(&self) -> usize {
        self.index
    }

    fn set_index(&mut self, index: usize) {
        self.index = index;
    }

    fn canvas_at_mut(&mut self, index: usize) -> Option<&mut TheCanvas> {
        if index < self.canvas.len() {
            Some(&mut self.canvas[index])
        } else {
            None
        }
    }
}
