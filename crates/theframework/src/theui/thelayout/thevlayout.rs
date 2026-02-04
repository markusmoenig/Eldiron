use crate::prelude::*;

/// The layout mode.
#[derive(PartialEq, Clone, Debug)]
pub enum TheVLayoutMode {
    /// Lays out the content based on their limiter settings (the default).
    ContentBased,
    /// Distributes the content evenly based on the available space.
    SizeBased,
}

pub struct TheVLayout {
    id: TheId,
    limiter: TheSizeLimiter,

    mode: TheVLayoutMode,
    dim: TheDim,

    widgets: Vec<Box<dyn TheWidget>>,

    margin: Vec4<i32>,
    padding: i32,

    background: Option<TheThemeColors>,
    reverse_index: Option<i32>,

    alignment: TheHorizontalAlign,
}

impl TheLayout for TheVLayout {
    fn new(id: TheId) -> Self
    where
        Self: Sized,
    {
        Self {
            id,
            limiter: TheSizeLimiter::new(),

            mode: TheVLayoutMode::ContentBased,
            dim: TheDim::zero(),

            widgets: vec![],

            margin: Vec4::new(10, 10, 10, 10),
            padding: 5,

            background: Some(DefaultWidgetBackground),
            reverse_index: None,

            alignment: TheHorizontalAlign::Center,
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

    fn get_widget_at_coord(&mut self, coord: Vec2<i32>) -> Option<&mut Box<dyn TheWidget>> {
        let widgets = self.widgets();
        widgets.iter_mut().find(|w| w.dim().contains(coord))
    }

    fn get_widget(
        &mut self,
        name: Option<&String>,
        uuid: Option<&Uuid>,
    ) -> Option<&mut Box<dyn TheWidget>> {
        self.widgets.iter_mut().find(|w| w.id().matches(name, uuid))
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

            if self.mode == TheVLayoutMode::ContentBased {
                let mut to_go = self.widgets.len();
                if let Some(split_index) = self.reverse_index {
                    to_go -= split_index as usize;
                }

                let mut y = self.margin.y;
                for i in 0..to_go {
                    self.widgets[i].calculate_size(ctx);
                    let width = self.widgets[i].limiter().get_width(dim.width);
                    let height = self.widgets[i].limiter().get_height(dim.height);

                    // Limit to visible area
                    if y + height > dim.height {
                        break;
                    }

                    let mut x = self.margin.x;
                    if self.alignment == TheHorizontalAlign::Center
                        && self.dim.width > self.margin.x + self.margin.z
                    {
                        let off = (self.dim.width - self.margin.x - self.margin.z - width) / 2;
                        if x + off + width < self.dim.width {
                            x += off;
                        }
                    }

                    self.widgets[i].set_dim(TheDim::new(dim.x + x, dim.y + y, width, height), ctx);
                    self.widgets[i]
                        .dim_mut()
                        .set_buffer_offset(self.dim.buffer_x + x, self.dim.buffer_y + y);
                    y += height + self.padding;
                }

                if let Some(reverse) = self.reverse_index {
                    let mut y: i32 = self.dim.height - self.margin.w;

                    for i in 0..reverse {
                        let i: usize = self.widgets.len() - 1 - i as usize;

                        self.widgets[i].calculate_size(ctx);
                        let width = self.widgets[i].limiter().get_width(dim.width);
                        let height = self.widgets[i].limiter().get_height(dim.height);

                        y -= height;
                        // Limit to visible area
                        if y + height > dim.height {
                            break;
                        }

                        let mut x = self.margin.x;
                        if
                        /*self.alignment == TheHorizontalAlign::Center
                        &&*/
                        self.dim.width > self.margin.x + self.margin.z {
                            let off = (self.dim.width - self.margin.x - self.margin.z - width) / 2;
                            if x + off + width < self.dim.width {
                                x += off;
                            }
                        }

                        self.widgets[i]
                            .set_dim(TheDim::new(dim.x + x, dim.y + y, width, height), ctx);
                        self.widgets[i]
                            .dim_mut()
                            .set_buffer_offset(self.dim.buffer_x + x, self.dim.buffer_y + y);
                        y -= self.padding;
                    }
                }
            } else if self.mode == TheVLayoutMode::SizeBased {
                let count = self.widgets.len() as i32;
                let total_height =
                    dim.height - self.margin.y - self.margin.w - (count - 1) * self.padding;
                let height = total_height / count;
                let width = dim.width - self.margin.x - self.margin.z;
                let mut y = self.margin.y;

                for w in &mut self.widgets {
                    w.calculate_size(ctx);

                    // Limit to visible area
                    if y + height > dim.height {
                        break;
                    }

                    let x = self.margin.x;

                    w.set_dim(TheDim::new(dim.x + x, dim.y + y, width, height), ctx);
                    w.dim_mut()
                        .set_buffer_offset(self.dim.buffer_x + x, self.dim.buffer_y + y);
                    y += height + self.padding;
                }
            }
        }
    }

    fn relayout(&mut self, ctx: &mut TheContext) {
        let dim = self.dim;
        self.dim = TheDim::zero();
        self.set_dim(dim, ctx);
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
        if let Some(background) = self.background {
            let stride = buffer.stride();

            ctx.draw.rect(
                buffer.pixels_mut(),
                &self.dim.to_buffer_utuple(),
                stride,
                style.theme().color(background),
            );
        }

        for w in &mut self.widgets {
            w.draw(buffer, style, ctx);
        }
    }

    fn as_vlayout(&mut self) -> Option<&mut dyn TheVLayoutTrait> {
        Some(self)
    }
}

/// TheVLayout specific functions.
pub trait TheVLayoutTrait {
    /// Clear the layout.
    fn clear(&mut self);
    /// Add a widget to the layout.
    fn add_widget(&mut self, widget: Box<dyn TheWidget>);
    /// Set the layout mode.
    fn set_mode(&mut self, mode: TheVLayoutMode);
    /// Set the top / bottom alingnment split index
    fn set_reverse_index(&mut self, reverse_index: Option<i32>);
    /// Set the horizontal alignment.
    fn set_alignment(&mut self, align: TheHorizontalAlign);
}

impl TheVLayoutTrait for TheVLayout {
    fn clear(&mut self) {
        self.widgets = vec![];
    }
    fn add_widget(&mut self, widget: Box<dyn TheWidget>) {
        self.widgets.push(widget);
    }
    fn set_mode(&mut self, mode: TheVLayoutMode) {
        self.mode = mode;
    }
    fn set_reverse_index(&mut self, reverse_index: Option<i32>) {
        self.reverse_index = reverse_index;
    }
    fn set_alignment(&mut self, align: TheHorizontalAlign) {
        self.alignment = align;
    }
}
