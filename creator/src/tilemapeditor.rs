use shared::tilemap;

use crate::prelude::*;

pub struct TilemapEditor {}

#[allow(clippy::new_without_default)]
impl TilemapEditor {
    pub fn new() -> Self {
        Self {}
    }

    pub fn build(&self) -> TheCanvas {
        let mut canvas = TheCanvas::new();

        let rgba_layout = TheRGBALayout::new(TheId::named("Tilemap Editor"));

        canvas.set_layout(rgba_layout);

        //

        let mut toolbar_canvas = TheCanvas::new();
        let traybar_widget = TheTraybar::new(TheId::empty());
        toolbar_canvas.set_widget(traybar_widget);

        let mut add_button = TheTraybarButton::new(TheId::named("Tilemap Editor Add Selection"));
        add_button.set_text("Add Tile".to_string());

        let mut clear_button =
            TheTraybarButton::new(TheId::named("Tilemap Editor Clear Selection"));
        clear_button.set_text("Clear".to_string());

        //let icon_view = TheIconView::new(TheId::named("Tilemap Editor Icon View"));

        let mut tile_name_text = TheText::new(TheId::empty());
        tile_name_text.set_text("Tile Name".to_string());

        let mut tile_name_edit = TheTextLineEdit::new(TheId::named("Tilemap Editor Name Edit"));
        tile_name_edit.limiter_mut().set_max_width(150);

        let mut block_name_text = TheText::new(TheId::empty());
        block_name_text.set_text("Blocking".to_string());

        let block_check_button = TheCheckButton::new(TheId::named("Tilemap Editor Block"));

        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(vec4i(10, 4, 5, 4));

        //toolbar_hlayout.add_widget(Box::new(icon_view));
        //let mut hdivider = TheHDivider::new(TheId::empty());
        //hdivider.limiter_mut().set_max_width(15);
        //toolbar_hlayout.add_widget(Box::new(hdivider));

        toolbar_hlayout.add_widget(Box::new(tile_name_text));
        toolbar_hlayout.add_widget(Box::new(tile_name_edit));

        let mut hdivider = TheHDivider::new(TheId::empty());
        hdivider.limiter_mut().set_max_width(15);
        toolbar_hlayout.add_widget(Box::new(hdivider));

        let mut drop_down = TheDropdownMenu::new(TheId::named("Tilemap Editor Role"));

        for dir in TileRole::iterator() {
            drop_down.add_option(dir.to_string().to_string());
            /*
            let mut color_button = TheColorButton::new(TheId::named(
                "Tilemap Editor Filter Character",
            ));
            color_button.limiter_mut().set_max_size(vec2i(19, 19));
            color_button.set_color(dir.to_color().to_u8_array());
            if dir == TileRole::Character {
                color_button.set_state(TheWidgetState::Selected);
            }
            toolbar_hlayout.add_widget(Box::new(color_button));*/
        }
        toolbar_hlayout.add_widget(Box::new(drop_down));

        let mut hdivider = TheHDivider::new(TheId::empty());
        hdivider.limiter_mut().set_max_width(15);
        toolbar_hlayout.add_widget(Box::new(hdivider));

        toolbar_hlayout.add_widget(Box::new(block_name_text));
        toolbar_hlayout.add_widget(Box::new(block_check_button));

        let mut hdivider = TheHDivider::new(TheId::empty());
        hdivider.limiter_mut().set_max_width(15);
        toolbar_hlayout.add_widget(Box::new(hdivider));

        toolbar_hlayout.add_widget(Box::new(add_button));
        toolbar_hlayout.add_widget(Box::new(clear_button));
        toolbar_hlayout.set_reverse_index(Some(1));

        toolbar_canvas.set_layout(toolbar_hlayout);
        canvas.set_top(toolbar_canvas);

        canvas
    }

    pub fn set_tilemap(&mut self, tilemap: &tilemap::Tilemap, ui: &mut TheUI, _: &mut TheContext) {
        if let Some(rgba_layout) = ui.get_rgba_layout("Tilemap Editor") {
            rgba_layout.set_buffer(tilemap.buffer.clone());
            rgba_layout.set_scroll_offset(tilemap.scroll_offset);
            if let Some(rgba_view) = rgba_layout.rgba_view_mut().as_rgba_view() {
                rgba_view.set_grid(Some(tilemap.grid_size));
                rgba_view.set_mode(TheRGBAViewMode::TileSelection);
            }
        }
    }

    // pub fn handle_event(&mut self, event: &TheEvent, ui: &mut TheUI, ctx: &mut TheContext) -> bool {
    //     let redraw = false;

    //     match event {
    //         // TheEvent::TilePicked(id, pos) => {
    //         //     if id.name == self.make_id(" RGBA Layout View") {
    //         //         if let Some(tile_id) = self.tile_ids.get(&(pos.x, pos.y)) {
    //         //             ctx.ui.send(TheEvent::StateChanged(TheId::named_with_id("Tilemap Tile", *tile_id), TheWidgetState::Selected));
    //         //         }
    //         //     }
    //         // }
    //         TheEvent::ValueChanged(_id, _value) => {}
    //         _ => {}
    //     }
    //     redraw
    // }
}
