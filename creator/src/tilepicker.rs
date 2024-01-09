use crate::prelude::*;

pub struct TilePicker {
    pub id: String,

    pub tile_ids: FxHashMap<(i32, i32), Uuid>,
    pub tile_text: FxHashMap<(i32, i32), String>,

    pub filter: String,
    pub filter_role: u8,
    pub zoom: f32,
}

#[allow(clippy::new_without_default)]
impl TilePicker {
    pub fn new(id: String) -> Self {
        Self {
            id,
            tile_ids: FxHashMap::default(),
            tile_text: FxHashMap::default(),
            filter: "".to_string(),
            filter_role: 0,
            zoom: 1.0,
        }
    }

    pub fn build(&self, minimal: bool) -> TheCanvas {
        let mut canvas = TheCanvas::new();

        // Toolbar
        let mut toolbar_canvas = TheCanvas::default();
        let traybar_widget = TheTraybar::new(TheId::empty());
        toolbar_canvas.set_widget(traybar_widget);
        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.set_background_color(None);

        let mut filter_text = TheText::new(TheId::empty());
        filter_text.set_text("Filter".to_string());

        toolbar_hlayout.set_margin(vec4i(10, 1, 5, 1));
        toolbar_hlayout.set_padding(3);
        toolbar_hlayout.add_widget(Box::new(filter_text));
        let mut filter_edit = TheTextLineEdit::new(TheId::named(&self.make_id(" Filter Edit")));
        filter_edit.set_text("".to_string());
        filter_edit
            .limiter_mut()
            .set_max_size(vec2i(if minimal { 75 } else { 120 }, 18));
        filter_edit.set_font_size(12.5);
        filter_edit.set_embedded(true);
        filter_edit.set_status_text("Show tiles containing the given text.");
        filter_edit.set_continuous(true);
        toolbar_hlayout.add_widget(Box::new(filter_edit));

        if !minimal {
            let mut spacer = TheSpacer::new(TheId::empty());
            spacer.limiter_mut().set_max_width(10);
            toolbar_hlayout.add_widget(Box::new(spacer));
        }

        // for dir in TileRole::iterator() {
        //     let mut color_button = TheColorButton::new(TheId::named("Tilemap Filter Character"));
        //     color_button.limiter_mut().set_max_size(vec2i(17, 17));
        //     color_button.set_color(dir.to_color().to_u8_array());
        //     color_button.set_state(TheWidgetState::Selected);
        //     color_button.set_status_text(format!("Show \"{}\" tiles.", dir.to_string()).as_str());
        //     toolbar_hlayout.add_widget(Box::new(color_button));
        // }

        let mut drop_down = TheDropdownMenu::new(TheId::named(&self.make_id(" Filter Role")));
        drop_down.add_option("All".to_string());
        for dir in TileRole::iterator() {
            drop_down.add_option(dir.to_string().to_string());
        }
        toolbar_hlayout.add_widget(Box::new(drop_down));

        if !minimal {
            let mut zoom = TheSlider::new(TheId::named(&self.make_id(" Zoom")));
            zoom.set_value(TheValue::Float(1.0));
            zoom.set_range(TheValue::RangeF32(1.0..=3.0));
            zoom.set_continuous(true);
            zoom.limiter_mut().set_max_width(120);
            toolbar_hlayout.add_widget(Box::new(zoom));
            toolbar_hlayout.set_reverse_index(Some(1));
        }

        toolbar_canvas.set_layout(toolbar_hlayout);

        // Canvas
        let mut rgba_layout = TheRGBALayout::new(TheId::named(&self.make_id(" RGBA Layout")));
        if let Some(rgba_view) = rgba_layout.rgba_view_mut().as_rgba_view() {
            rgba_view.set_grid(Some(24));
            rgba_view.set_mode(TheRGBAViewMode::TilePicker);
            let mut c = WHITE;
            c[3] = 128;
            rgba_view.set_hover_color(Some(c));
        }

        canvas.set_top(toolbar_canvas);
        canvas.set_layout(rgba_layout);

        canvas
    }

    /// Set the tiles for the picker.
    pub fn set_tiles(&mut self, tiles: Vec<TheRGBATile>, ui: &mut TheUI, ctx: &mut TheContext) {
        self.tile_ids.clear();
        self.tile_text.clear();
        if let Some(editor) = ui.get_rgba_layout(&self.make_id(" RGBA Layout")) {
            //println!("{}", editor.dim().width);
            let width = editor.dim().width - 16;
            let height = editor.dim().height - 16;

            if let Some(rgba_view) = editor.rgba_view_mut().as_rgba_view() {
                let grid = (36_f32 * self.zoom) as i32;

                rgba_view.set_grid(Some(grid));

                let mut filtered_tiles = vec![];

                for t in tiles {
                    if t.name.to_lowercase().contains(&self.filter)
                        && (self.filter_role == 0 || t.role == self.filter_role - 1)
                    {
                        filtered_tiles.push(t);
                    }
                }

                let tiles_per_row = width / grid;
                let lines = filtered_tiles.len() as i32 / tiles_per_row + 1;

                let mut buffer =
                    TheRGBABuffer::new(TheDim::sized(width, max(lines * grid, height)));

                for (i, tile) in filtered_tiles.iter().enumerate() {
                    let x = i as i32 % tiles_per_row;
                    let y = i as i32 / tiles_per_row;

                    self.tile_ids.insert((x, y), tile.id);
                    self.tile_text.insert(
                        (x, y),
                        format!(
                            "{} : {}",
                            tile.name,
                            TileRole::from_index(tile.role)
                                .unwrap_or(TileRole::ManMade)
                                .to_string()
                        ),
                    );
                    if !tile.buffer.is_empty() {
                        buffer.copy_into(x * grid, y * grid, &tile.buffer[0]);
                        buffer.copy_into(x * grid, y * grid, &tile.buffer[0].scaled(grid, grid));
                    }
                }

                rgba_view.set_buffer(buffer);
            }
            editor.relayout(ctx);
        }
    }

    pub fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
    ) -> bool {
        let redraw = false;

        match event {
            TheEvent::TilePicked(id, pos) => {
                if id.name == self.make_id(" RGBA Layout View") {
                    if let Some(tile_id) = self.tile_ids.get(&(pos.x, pos.y)) {
                        ctx.ui.send(TheEvent::StateChanged(
                            TheId::named_with_id("Tilemap Tile", *tile_id),
                            TheWidgetState::Selected,
                        ));
                    }
                }
            }
            TheEvent::TileEditorHoverChanged(id, pos) => {
                if id.name == self.make_id(" RGBA Layout View") {
                    ctx.ui.send(TheEvent::SetStatusText(
                        id.clone(),
                        self.tile_text
                            .get(&(pos.x, pos.y))
                            .unwrap_or(&"".to_string())
                            .to_string(),
                    ));
                }
            }
            TheEvent::TileEditorDelete(id, selected) => {
                if id.name == self.make_id(" RGBA Layout View") {
                    for tile_pos in selected {
                        if let Some(tile_id) = self.tile_ids.get(tile_pos) {
                            project.remove_tile(tile_id);
                        }
                    }
                    self.set_tiles(project.extract_tiles_vec(), ui, ctx);
                }
            }
            TheEvent::Custom(id, _value) => {
                if id.name == "Update Tilepicker" {
                    self.set_tiles(project.extract_tiles_vec(), ui, ctx);
                }
            }
            TheEvent::ValueChanged(id, value) => {
                if id.name == self.make_id(" Filter Edit") {
                    if let TheValue::Text(filter) = value {
                        self.filter = filter.to_lowercase();
                        self.set_tiles(project.extract_tiles_vec(), ui, ctx);
                    }
                } else if id.name == self.make_id(" Filter Role") {
                    if let TheValue::Int(filter) = value {
                        self.filter_role = *filter as u8;
                        self.set_tiles(project.extract_tiles_vec(), ui, ctx);
                    }
                } else if id.name == self.make_id(" Zoom") {
                    if let TheValue::Float(zoom) = value {
                        self.zoom = *zoom;
                        self.set_tiles(project.extract_tiles_vec(), ui, ctx);
                    }
                }
            }
            _ => {}
        }
        redraw
    }

    ///  Create an id.
    fn make_id(&self, id: &str) -> String {
        self.id.to_owned() + id
    }
}
