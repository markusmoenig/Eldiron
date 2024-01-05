use serde_json::value;

use crate::prelude::*;

pub struct TilePicker {
    pub id: String,

    pub tile_ids: FxHashMap<(i32, i32), Uuid>,
}

#[allow(clippy::new_without_default)]
impl TilePicker {
    pub fn new(id: String) -> Self {
        Self {
            id,
            tile_ids: FxHashMap::default(),
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
        let mut filter_edit = TheTextLineEdit::new(TheId::named("Tilemap Filter Edit"));
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

        for dir in TileRole::iterator() {
            let mut color_button = TheColorButton::new(TheId::named("Tilemap Filter Character"));
            color_button.limiter_mut().set_max_size(vec2i(17, 17));
            color_button.set_color(dir.to_color().to_u8_array());
            color_button.set_state(TheWidgetState::Selected);
            color_button.set_status_text(format!("Show \"{}\" tiles.", dir.to_string()).as_str());
            toolbar_hlayout.add_widget(Box::new(color_button));
        }

        if !minimal {
            let mut zoom = TheSlider::new(TheId::named("Region Editor Zoom"));
            zoom.set_value(TheValue::Float(1.0));
            zoom.set_range(TheValue::RangeF32(0.3..=3.0));
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
        }

        canvas.set_top(toolbar_canvas);
        canvas.set_layout(rgba_layout);

        canvas
    }

    /// Set the tiles for the picker.
    pub fn set_tiles(&mut self, tiles: &Vec<TheRGBATile>, ui: &mut TheUI) {

        self.tile_ids.clear();
        if let Some(editor) = ui.get_rgba_layout(&self.make_id(" RGBA Layout")) {
            //println!("{}", editor.dim().width);
            let width = editor.dim().width - 24;
            let height = editor.dim().height - 24;

            if let Some(rgba_view) = editor.rgba_view_mut().as_rgba_view() {
                let grid = 24;

                let tiles_per_row = width / grid;
                let lines = max(tiles.len() as i32 / tiles_per_row, 1);

                let mut buffer = TheRGBABuffer::new(TheDim::sized(width, max(lines * grid, height)));

                for (i, tile) in tiles.iter().enumerate() {
                    let x = (i as i32 % tiles_per_row) * grid;
                    let y = (i as i32 / tiles_per_row) * grid;

                    self.tile_ids.insert((x, y), tile.id);
                    if !tile.buffer.is_empty() {
                        buffer.copy_into(x, y, &tile.buffer[0]);
                    }
                }

                rgba_view.set_buffer(buffer);
            }
        }
    }

    pub fn handle_event(&mut self, event: &TheEvent, ui: &mut TheUI, ctx: &mut TheContext) -> bool {
        let redraw = false;

        match event {
            TheEvent::TilePicked(id, pos) => {
                if id.name == self.make_id(" RGBA Layout View") {
                    if let Some(tile_id) = self.tile_ids.get(&(pos.x, pos.y)) {
                        ctx.ui.send(TheEvent::StateChanged(TheId::named_with_id("Tilemap Tile", *tile_id), TheWidgetState::Selected));
                    }
                }
            }
            TheEvent::ValueChanged(_id, _value) => {}
            _ => {}
        }
        redraw
    }

    ///  Create an id.
    fn make_id(&self, id: &str) -> String {
        self.id.to_owned() + id
    }
}
