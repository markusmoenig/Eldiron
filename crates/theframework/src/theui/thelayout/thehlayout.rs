use crate::prelude::*;

/// The layout mode.
#[derive(PartialEq, Clone, Debug)]
pub enum TheHLayoutMode {
    /// Lays out the content based on their limiter settings (the default).
    ContentBased,
    /// Distributes the content evenly based on the available space.
    SizeBased,
}

pub struct TheHLayout {
    id: TheId,
    limiter: TheSizeLimiter,

    mode: TheHLayoutMode,
    dim: TheDim,

    widgets: Vec<Box<dyn TheWidget>>,

    margin: Vec4<i32>,
    padding: i32,

    background: Option<TheThemeColors>,
    reverse_index: Option<i32>,

    redirect_as: Option<TheId>,
}

impl TheLayout for TheHLayout {
    fn new(id: TheId) -> Self
    where
        Self: Sized,
    {
        Self {
            id,
            limiter: TheSizeLimiter::new(),
            mode: TheHLayoutMode::ContentBased,

            dim: TheDim::zero(),

            widgets: vec![],

            margin: Vec4::new(10, 10, 10, 10),
            padding: 5,

            background: Some(DefaultWidgetBackground),
            reverse_index: None,

            redirect_as: None,
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
        self.widgets.iter_mut().find(|w| w.dim().contains(coord))
    }

    fn get_widget(
        &mut self,
        name: Option<&String>,
        uuid: Option<&Uuid>,
    ) -> Option<&mut Box<dyn TheWidget>> {
        self.widgets
            .iter_mut()
            .find(|w: &&mut Box<dyn TheWidget>| w.id().matches(name, uuid))
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

            if !self.widgets.is_empty() {
                let mut x: i32 = self.margin.x;

                let mut to_go = self.widgets.len();
                if let Some(split_index) = self.reverse_index {
                    to_go -= split_index as usize;
                }

                if self.mode == TheHLayoutMode::ContentBased {
                    for i in 0..to_go {
                        self.widgets[i].calculate_size(ctx);
                        let width = self.widgets[i].limiter().get_width(dim.width);
                        let height = self.widgets[i].limiter().get_height(dim.height);

                        // Limit to visible area
                        if x + width > dim.width {
                            break;
                        }

                        let mut y = self.margin.y;
                        if self.dim.height > self.margin.y + self.margin.w {
                            let mut off =
                                (self.dim.height - self.margin.y - self.margin.w - height) / 2;
                            if self.widgets[i].as_text().is_some() {
                                off -= 1;
                            }
                            if y + off + height < self.dim.height {
                                y += off;
                            }
                        }

                        self.widgets[i]
                            .set_dim(TheDim::new(dim.x + x, dim.y + y, width, height), ctx);
                        self.widgets[i]
                            .dim_mut()
                            .set_buffer_offset(self.dim.buffer_x + x, self.dim.buffer_y + y);
                        x += width + self.padding;
                    }

                    if let Some(reverse) = self.reverse_index {
                        let mut x: i32 = self.dim.width - self.margin.z;

                        for i in 0..reverse {
                            let i = self.widgets.len() - 1 - i as usize;

                            self.widgets[i].calculate_size(ctx);
                            let width = self.widgets[i].limiter().get_width(dim.width);
                            let height = self.widgets[i].limiter().get_height(dim.height);

                            x -= width;
                            // Limit to visible area
                            if x + width > dim.width {
                                break;
                            }

                            let mut y = self.margin.y;
                            if self.dim.height > self.margin.y + self.margin.w {
                                let mut off =
                                    (self.dim.height - self.margin.y - self.margin.w - height) / 2;
                                if self.widgets[i].as_text().is_some() {
                                    off -= 1;
                                }
                                if y + off + height < self.dim.height {
                                    y += off;
                                }
                            }

                            self.widgets[i]
                                .set_dim(TheDim::new(dim.x + x, dim.y + y, width, height), ctx);
                            self.widgets[i]
                                .dim_mut()
                                .set_buffer_offset(self.dim.buffer_x + x, self.dim.buffer_y + y);
                            x -= self.padding;
                        }
                    }
                } else if self.mode == TheHLayoutMode::SizeBased {
                    let count = self.widgets.len() as i32;
                    let total_width =
                        dim.width - self.margin.x - self.margin.z - (count - 1) * self.padding;
                    let width = total_width / count;
                    let height = dim.height - self.margin.y - self.margin.w;

                    for w in &mut self.widgets {
                        w.calculate_size(ctx);

                        // Limit to visible area
                        if x + width > dim.width {
                            break;
                        }

                        let y = self.margin.y;

                        w.set_dim(TheDim::new(dim.x + x, dim.y + y, width, height), ctx);
                        w.dim_mut()
                            .set_buffer_offset(self.dim.buffer_x + x, self.dim.buffer_y + y);
                        x += width + self.padding;
                    }
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

    fn redirected_widget_value(
        &mut self,
        widget_id: &TheId,
        value: &TheValue,
        ctx: &mut TheContext,
    ) {
        //println!("redirected_widget_value: {:?}", widget_id);
        if let Some(id) = &self.redirect_as {
            if widget_id.name == "Int2 X" {
                if let Some(v) = value.to_i32() {
                    if let Some(y) = self.widgets[3].value().to_i32() {
                        ctx.ui.send(TheEvent::ValueChanged(
                            id.clone(),
                            TheValue::Int2(Vec2::new(v, y)),
                        ));
                    }
                }
            } else if widget_id.name == "Int2 Y" {
                if let Some(v) = value.to_i32() {
                    if let Some(x) = self.widgets[1].value().to_i32() {
                        ctx.ui.send(TheEvent::ValueChanged(
                            id.clone(),
                            TheValue::Int2(Vec2::new(x, v)),
                        ));
                    }
                }
            } else if widget_id.name == "Float2 X" {
                if let Some(v) = value.to_f32() {
                    if let Some(y) = self.widgets[3].value().to_f32() {
                        ctx.ui.send(TheEvent::ValueChanged(
                            id.clone(),
                            TheValue::Float2(Vec2::new(v, y)),
                        ));
                    }
                }
            } else if widget_id.name == "Float2 Y" {
                if let Some(v) = value.to_f32() {
                    if let Some(x) = self.widgets[1].value().to_f32() {
                        ctx.ui.send(TheEvent::ValueChanged(
                            id.clone(),
                            TheValue::Float2(Vec2::new(x, v)),
                        ));
                    }
                }
            }
        }
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

    fn as_hlayout(&mut self) -> Option<&mut dyn TheHLayoutTrait> {
        Some(self)
    }
}

/// TheHLayout specific functions.
pub trait TheHLayoutTrait: TheLayout {
    /// Add a widget to the layout.
    fn add_widget(&mut self, widget: Box<dyn TheWidget>);
    /// Set the layout mode.
    fn set_mode(&mut self, mode: TheHLayoutMode);
    /// Clear the layout.
    fn clear(&mut self);
    /// Set the redirection id.
    fn set_redirect_as(&mut self, id: TheId);
    /// Set the left / right alinment split index
    fn set_reverse_index(&mut self, reverse_index: Option<i32>);
}

impl TheHLayoutTrait for TheHLayout {
    fn add_widget(&mut self, widget: Box<dyn TheWidget>) {
        self.widgets.push(widget);
    }
    fn set_mode(&mut self, mode: TheHLayoutMode) {
        self.mode = mode;
    }
    fn clear(&mut self) {
        self.widgets = vec![];
    }
    fn set_redirect_as(&mut self, id: TheId) {
        self.redirect_as = Some(id);
    }
    fn set_reverse_index(&mut self, reverse_index: Option<i32>) {
        self.reverse_index = reverse_index;
    }
}
