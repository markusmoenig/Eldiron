use crate::prelude::*;

pub struct TheSnapperLayout {
    id: TheId,
    limiter: TheSizeLimiter,

    dim: TheDim,

    bars: Vec<Box<dyn TheWidget>>,

    layouts: Vec<Box<dyn TheLayout>>,
    widgets: Vec<Box<dyn TheWidget>>,

    margin: Vec4<i32>,

    background: Option<TheThemeColors>,
}

impl TheLayout for TheSnapperLayout {
    fn new(id: TheId) -> Self
    where
        Self: Sized,
    {
        Self {
            id,
            limiter: TheSizeLimiter::new(),

            dim: TheDim::zero(),

            bars: vec![],

            layouts: vec![],
            widgets: vec![],

            margin: Vec4::new(0, 0, 0, 0),

            background: Some(TextLayoutBackground),
        }
    }

    fn id(&self) -> &TheId {
        &self.id
    }

    fn set_margin(&mut self, margin: Vec4<i32>) {
        self.margin = margin;
    }

    fn set_background_color(&mut self, color: Option<TheThemeColors>) {
        self.background = color;
    }

    fn widgets(&mut self) -> &mut Vec<Box<dyn TheWidget>> {
        &mut self.widgets
    }

    fn get_widget_at_coord(&mut self, coord: Vec2<i32>) -> Option<&mut Box<dyn TheWidget>> {
        let mut open_states = vec![];

        for b in &mut self.bars {
            if b.is_open() {
                open_states.push(true);
            } else {
                open_states.push(false);
            }

            if b.dim().contains(coord) {
                return Some(b);
            }
        }

        for (index, l) in self.layouts.iter_mut().enumerate() {
            if open_states[index] {
                if let Some(w) = l.get_widget_at_coord(coord) {
                    return Some(w);
                }
            }
        }

        None
    }

    fn get_widget(
        &mut self,
        name: Option<&String>,
        uuid: Option<&Uuid>,
    ) -> Option<&mut Box<dyn TheWidget>> {
        let mut open_states = vec![];

        for b in &mut self.bars {
            if b.is_open() {
                open_states.push(true);
            } else {
                open_states.push(false);
            }

            if b.id().matches(name, uuid) {
                return Some(b);
            }
        }

        for (index, l) in self.layouts.iter_mut().enumerate() {
            if open_states[index] {
                let widgets = l.widgets();
                if let Some(w) = widgets.iter_mut().find(|w| w.id().matches(name, uuid)) {
                    return Some(w);
                }
            }
        }

        None
    }

    fn needs_redraw(&mut self) -> bool {
        for i in 0..self.bars.len() {
            if self.bars[i].needs_redraw() {
                return true;
            }
            if self.bars[i].is_open() && self.layouts[i].needs_redraw() {
                return true;
            }
        }
        false
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

            let x = self.margin.x;
            let mut y = self.margin.y;
            let width = dim.width;

            let sections = self.bars.len() as i32;
            let available_height = dim.height - sections * 22;

            let height_per_section = available_height / sections;

            for index in 0..sections {
                let i = index as usize;

                self.bars[i].set_dim(TheDim::new(dim.x + x, dim.y + y, width, 22), ctx);
                self.bars[i]
                    .dim_mut()
                    .set_buffer_offset(self.dim.buffer_x, self.dim.buffer_y + y);

                y += self.bars[i].dim().height;

                if self.bars[i].is_open() {
                    let mut dim = TheDim::new(dim.x + x, dim.y + y, width, height_per_section);
                    dim.buffer_x = self.dim.buffer_x;
                    dim.buffer_y = self.dim.buffer_y + y;
                    self.layouts[i].set_dim(dim, ctx);

                    y += height_per_section;
                }
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
        if !self.dim().is_valid() {
            return;
        }

        let stride = buffer.stride();
        let utuple: (usize, usize, usize, usize) = self.dim.to_buffer_utuple();

        ctx.draw.rect(
            buffer.pixels_mut(),
            &utuple,
            stride,
            style.theme().color(DefaultWidgetBackground),
        );

        let sections = self.bars.len();
        for i in 0..sections {
            self.bars[i].draw(buffer, style, ctx);
            if self.bars[i].is_open() {
                self.layouts[i].draw(buffer, style, ctx);
            }
        }
    }
}

/// TheSnapperLayout specific functions.
pub trait TheSnapperLayoutTrait {
    /// Add a snapperbar / layout pair.
    fn add_pair(&mut self, snapperbar: Box<dyn TheWidget>, layout: Box<dyn TheLayout>);
}

impl TheSnapperLayoutTrait for TheSnapperLayout {
    fn add_pair(&mut self, snapperbar: Box<dyn TheWidget>, layout: Box<dyn TheLayout>) {
        self.bars.push(snapperbar);
        self.layouts.push(layout);
    }
}
