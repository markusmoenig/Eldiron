use crate::prelude::*;

pub struct ThePalettePicker {
    id: TheId,
    limiter: TheSizeLimiter,

    is_dirty: bool,

    palette: ThePalette,
    index: usize,
    dynamic_layout: bool,

    rectangles: Vec<TheDim>,

    rows: i32,
    columns: i32,

    dim: TheDim,
}

impl TheWidget for ThePalettePicker {
    fn new(id: TheId) -> Self
    where
        Self: Sized,
    {
        let mut limiter = TheSizeLimiter::new();
        limiter.set_max_size(Vec2::new(200, 400));

        Self {
            id,
            limiter,

            is_dirty: true,

            palette: ThePalette::default(),
            index: 0,
            dynamic_layout: false,

            rectangles: vec![],

            rows: 20,
            columns: 14,

            dim: TheDim::zero(),
        }
    }

    fn id(&self) -> &TheId {
        &self.id
    }

    #[allow(clippy::single_match)]
    fn on_event(&mut self, event: &TheEvent, ctx: &mut TheContext) -> bool {
        let mut redraw = false;
        // println!("event ({}): {:?}", self.widget_id.name, event);
        match event {
            TheEvent::MouseDown(coord) => {
                ctx.ui
                    .send_widget_state_changed(self.id(), TheWidgetState::Clicked);
                ctx.ui.set_focus(self.id());

                self.is_dirty = true;
                redraw = true;

                for (i, rect) in self.rectangles.iter().enumerate() {
                    if rect.contains(*coord) {
                        self.index = i;
                        ctx.ui.send(TheEvent::PaletteIndexChanged(
                            self.id().clone(),
                            self.index as u16,
                        ));
                        break;
                    }
                }
            }
            TheEvent::KeyCodeDown(TheValue::KeyCode(code)) => match code {
                TheKeyCode::Delete => {
                    self.palette.colors[self.index] = None;
                    self.is_dirty = true;
                    redraw = true;
                }
                TheKeyCode::Left => {
                    if self.index > 0 {
                        self.index -= 1;
                        self.is_dirty = true;
                        redraw = true;
                        ctx.ui.send(TheEvent::PaletteIndexChanged(
                            self.id().clone(),
                            self.index as u16,
                        ));
                    }
                }
                TheKeyCode::Right => {
                    if self.index < self.palette.colors.len() - 1 {
                        self.index += 1;
                        self.is_dirty = true;
                        redraw = true;
                        ctx.ui.send(TheEvent::PaletteIndexChanged(
                            self.id().clone(),
                            self.index as u16,
                        ));
                    }
                }
                _ => {}
            },
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

    fn draw(
        &mut self,
        buffer: &mut TheRGBABuffer,
        style: &mut Box<dyn TheStyle>,
        ctx: &mut TheContext,
    ) {
        if !self.dim().is_valid() {
            return;
        }

        let utuple = self.dim.to_buffer_utuple();
        let stride = buffer.stride();

        ctx.draw.rect(
            buffer.pixels_mut(),
            &utuple,
            stride,
            style.theme().color(ListLayoutBackground),
        );

        let width = buffer.dim().width;
        let height = buffer.dim().height;

        let mut item_width = 18;
        let item_spacing = 1;

        if self.dynamic_layout {
            let (columns, rows, iw) =
                self.calc_layout(Vec2::new(width, height), self.palette.colors.len());
            item_width = iw as usize;
            self.rows = rows;
            self.columns = columns;
            // println!(
            //     "width {} height {} : columns {} rows {} item_width {}",
            //     width, height, columns, rows, item_width
            // );
        }

        self.rectangles.clear();

        let mut index = 0;
        let mut y_off = 8;
        for _ in 0..self.rows {
            let mut x_off = 10;
            for _ in 0..self.columns {
                if self.index == index {
                    ctx.draw.rect_outline(
                        buffer.pixels_mut(),
                        &(utuple.0 + x_off, utuple.0 + y_off, item_width, item_width),
                        stride,
                        &WHITE,
                    );
                }

                ctx.draw.rect_outline(
                    buffer.pixels_mut(),
                    &(
                        utuple.0 + x_off + 1,
                        utuple.0 + y_off + 1,
                        item_width - 2,
                        item_width - 2,
                    ),
                    stride,
                    &BLACK,
                );

                if let Some(Some(color)) = self.palette.colors.get(index) {
                    ctx.draw.rect(
                        buffer.pixels_mut(),
                        &(
                            utuple.0 + x_off + 2,
                            utuple.0 + y_off + 2,
                            item_width - 4,
                            item_width - 4,
                        ),
                        stride,
                        &color.to_u8_array(),
                    );
                }
                self.rectangles.push(TheDim::new(
                    x_off as i32,
                    y_off as i32,
                    item_width as i32,
                    item_width as i32,
                ));
                index += 1;
                x_off += item_width + item_spacing;
                if index >= self.palette.colors.len() {
                    break;
                }
            }
            if index >= self.palette.colors.len() {
                break;
            }
            y_off += item_width + item_spacing;
        }

        self.is_dirty = false;
    }

    fn as_palette_picker(&mut self) -> Option<&mut dyn ThePalettePickerTrait> {
        Some(self)
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub trait ThePalettePickerTrait {
    fn set_palette(&mut self, palette: ThePalette);
    fn index(&self) -> usize;
    fn set_color(&mut self, color: TheColor);
    fn set_rows_columns(&mut self, rows: i32, columns: i32);
    fn set_dynamic_layout(&mut self, dynamic_layout: bool);
    fn calc_layout(&self, available: Vec2<i32>, colour_count: usize) -> (i32, i32, i32);
}

impl ThePalettePickerTrait for ThePalettePicker {
    fn set_palette(&mut self, palette: ThePalette) {
        self.palette = palette;
        self.rectangles.clear();
        self.is_dirty = true;
    }
    fn index(&self) -> usize {
        self.index
    }
    fn set_color(&mut self, color: TheColor) {
        self.palette.colors[self.index] = Some(color);
        self.is_dirty = true;
    }
    fn set_rows_columns(&mut self, rows: i32, columns: i32) {
        self.rows = rows;
        self.columns = columns;
    }

    fn set_dynamic_layout(&mut self, dynamic_layout: bool) {
        self.dynamic_layout = dynamic_layout;
    }

    /// Returns (columns, rows, item_width) that best fill the area.
    fn calc_layout(&self, area: Vec2<i32>, count: usize) -> (i32, i32, i32) {
        const PAD_X: i32 = 10;
        const PAD_Y: i32 = 8;
        const SPACING: i32 = 1;
        const MIN_CELL: i32 = 8;

        if count == 0 {
            return (0, 0, 0);
        }

        let aw = (area.x - PAD_X * 2).max(MIN_CELL);
        let ah = (area.y - PAD_Y * 2).max(MIN_CELL);
        let max_cols = ((aw + SPACING) / (MIN_CELL + SPACING))
            .max(1)
            .min(count as i32);

        let mut best = (1, count as i32, MIN_CELL);

        for cols in 1..=max_cols {
            let rows = (count as i32 + cols - 1) / cols; // ceil
                                                         // width- and height-limited cell size for this grid
            let cell_w = (aw - (cols - 1) * SPACING) / cols;
            let cell_h = (ah - (rows - 1) * SPACING) / rows;
            let cell = cell_w.min(cell_h);

            if cell < MIN_CELL {
                continue;
            }

            if cell > best.2 || (cell == best.2 && cols > best.0) {
                best = (cols, rows, cell);
            }
        }

        best
    }
}
