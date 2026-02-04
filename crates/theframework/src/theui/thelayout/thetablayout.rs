use crate::prelude::*;

pub struct TheTabLayout {
    id: TheId,
    dim: TheDim,
    limiter: TheSizeLimiter,

    tabbar: Box<dyn TheWidget>,

    canvas: Vec<TheCanvas>,

    widgets: Vec<Box<dyn TheWidget>>,
    index: usize,
}

impl TheLayout for TheTabLayout {
    fn new(id: TheId) -> Self
    where
        Self: Sized,
    {
        Self {
            id: id.clone(),
            dim: TheDim::zero(),
            limiter: TheSizeLimiter::new(),

            tabbar: Box::new(TheTabbar::new(TheId::named((id.name + " Tabbar").as_str()))),

            canvas: vec![],
            widgets: vec![],
            index: 0,
        }
    }

    fn id(&self) -> &TheId {
        &self.id
    }

    fn set_margin(&mut self, _margin: Vec4<i32>) {}

    fn widgets(&mut self) -> &mut Vec<Box<dyn TheWidget>> {
        &mut self.widgets
    }

    fn get_widget_at_coord(&mut self, coord: Vec2<i32>) -> Option<&mut Box<dyn TheWidget>> {
        if self.tabbar.dim().contains(coord) {
            return Some(&mut self.tabbar);
        }

        if self.canvas.is_empty() {
            return None;
        }

        let mut index = 0;
        if let Some(tabbar) = self.tabbar.as_tabbar() {
            if let Some(i) = tabbar.selection_index() {
                index = i as usize;
            }
        }

        if index < self.canvas.len() {
            return self.canvas[index].get_widget_at_coord(coord);
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
        if self.tabbar.id().matches(name, uuid) {
            return Some(&mut self.tabbar);
        }

        if self.canvas.is_empty() {
            return None;
        }

        for canvas in &mut self.canvas {
            if let Some(widget) = canvas.get_widget(name, uuid) {
                return Some(widget);
            }
        }

        None
    }

    fn needs_redraw(&mut self) -> bool {
        if self.tabbar.needs_redraw() {
            return true;
        }

        // if self.widgets.is_empty() {
        //     return false;
        // }

        false
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

            self.tabbar
                .set_dim(TheDim::new(dim.x, dim.y, dim.width, 22), ctx);

            self.tabbar
                .dim_mut()
                .set_buffer_offset(self.dim.buffer_x, self.dim.buffer_y);

            for c in &mut self.canvas {
                c.set_dim(
                    TheDim::new(dim.x + 1, dim.y + 23, dim.width - 2, dim.height - 22 - 2),
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
        let stride = buffer.stride();
        let utuple: (usize, usize, usize, usize) = self.dim.to_buffer_utuple();

        ctx.draw.rect(
            buffer.pixels_mut(),
            &utuple,
            stride,
            style.theme().color(TabbarConnector),
        );

        self.tabbar.draw(buffer, style, ctx);

        if self.canvas.is_empty() {
            return;
        }

        let mut index = 0;
        if let Some(tabbar) = self.tabbar.as_tabbar() {
            if let Some(i) = tabbar.selection_index() {
                index = i as usize;
            }
        }

        if index < self.canvas.len() {
            self.canvas[index].draw(style, ctx);
            buffer.copy_into(
                self.dim.buffer_x + 1,
                self.dim.buffer_y + 23,
                self.canvas[index].buffer(),
            );
        }
    }

    /// Convert to the tab layout trait
    fn as_tab_layout(&mut self) -> Option<&mut dyn TheTabLayoutTrait> {
        Some(self)
    }
}

/// TheTabLayoutTrait specific functions.
pub trait TheTabLayoutTrait: TheLayout {
    /// Clear the canvas.
    fn clear(&mut self);
    /// Add a canvas to the stack.
    fn add_canvas(&mut self, name: String, canvas: TheCanvas);
    /// Returns the index of the current canvas.
    fn index(&self) -> usize;
    /// Set the index of the current canvas.
    fn set_index(&mut self, index: usize);
}

impl TheTabLayoutTrait for TheTabLayout {
    fn clear(&mut self) {
        if let Some(tabbar) = self.tabbar.as_tabbar() {
            tabbar.clear();
        }
        self.canvas.clear();
    }

    fn add_canvas(&mut self, name: String, canvas: TheCanvas) {
        if let Some(tabbar) = self.tabbar.as_tabbar() {
            tabbar.add_tab(name);
        }
        self.canvas.push(canvas);
    }

    fn index(&self) -> usize {
        self.index
    }

    fn set_index(&mut self, index: usize) {
        self.index = index;
        if let Some(tabbar) = self.tabbar.as_tabbar() {
            tabbar.set_selection_index(index);
        }
    }
}
