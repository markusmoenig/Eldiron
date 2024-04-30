use shared::tilemap;

use crate::prelude::*;

pub struct TilemapEditor {
    curr_tilemap_id: Uuid,
    anim_preview: bool,
}

#[allow(clippy::new_without_default)]
impl TilemapEditor {
    pub fn new() -> Self {
        Self {
            curr_tilemap_id: Uuid::new_v4(),
            anim_preview: true,
        }
    }

    pub fn build(&self) -> TheCanvas {
        let mut canvas = TheCanvas::new();

        let rgba_layout = TheRGBALayout::new(TheId::named("Tilemap Editor"));

        canvas.set_layout(rgba_layout);

        //

        let mut toolbar_canvas = TheCanvas::new();
        let traybar_widget = TheTraybar::new(TheId::empty());
        toolbar_canvas.set_widget(traybar_widget);

        let mut add_anim_button = TheTraybarButton::new(TheId::named("Tilemap Editor Add Anim"));
        add_anim_button.set_text("As Anim".to_string());
        add_anim_button.set_status_text(
            "Adds the current selection as animation, every tile is one animation frame.",
        );

        let mut add_multi_button = TheTraybarButton::new(TheId::named("Tilemap Editor Add Multi"));
        add_multi_button.set_text("As Multi".to_string());
        add_multi_button.set_status_text(
            "Adds the current selection as one big tile, it may consist of several smaller tiles.",
        );

        let mut clear_button = TheTraybarButton::new(TheId::named("Tilemap Editor Clear"));
        clear_button.set_text("Clear".to_string());
        clear_button.set_status_text("Clear the current selection.");

        //let icon_view = TheIconView::new(TheId::named("Tilemap Editor Icon View"));

        let mut tile_name_text = TheText::new(TheId::empty());
        tile_name_text.set_text("Tags".to_string());

        let mut tile_name_edit = TheTextLineEdit::new(TheId::named("Tilemap Editor Name Edit"));
        tile_name_edit.limiter_mut().set_max_width(130);

        let mut block_name_text = TheText::new(TheId::empty());
        block_name_text.set_text("Blocking".to_string());

        let block_check_button: TheCheckButton =
            TheCheckButton::new(TheId::named("Tilemap Editor Block"));

        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(vec4i(10, 4, 5, 4));

        toolbar_hlayout.add_widget(Box::new(tile_name_text));
        toolbar_hlayout.add_widget(Box::new(tile_name_edit));

        let mut hdivider = TheHDivider::new(TheId::empty());
        hdivider.limiter_mut().set_max_width(15);
        toolbar_hlayout.add_widget(Box::new(hdivider));

        let mut drop_down = TheDropdownMenu::new(TheId::named("Tilemap Editor Role"));

        for dir in TileRole::iterator() {
            drop_down.add_option(dir.to_string().to_string());
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

        toolbar_hlayout.add_widget(Box::new(add_anim_button));
        toolbar_hlayout.add_widget(Box::new(add_multi_button));

        let mut hdivider = TheHDivider::new(TheId::empty());
        hdivider.limiter_mut().set_max_width(15);
        toolbar_hlayout.add_widget(Box::new(hdivider));

        let mut zoom = TheSlider::new(TheId::named("Tilemap Editor Zoom"));
        zoom.set_value(TheValue::Float(1.0));
        zoom.set_range(TheValue::RangeF32(0.5..=3.0));
        zoom.set_continuous(true);
        zoom.limiter_mut().set_max_width(120);
        toolbar_hlayout.add_widget(Box::new(zoom));
        toolbar_hlayout.add_widget(Box::new(clear_button));
        toolbar_hlayout.set_reverse_index(Some(2));

        // Details
        let mut details_canvas = TheCanvas::new();

        let mut vlayout = TheVLayout::new(TheId::named(" Tile Details Layout"));
        vlayout.set_margin(vec4i(5, 20, 5, 10));
        vlayout.set_alignment(TheHorizontalAlign::Center);
        vlayout.limiter_mut().set_max_width(120);

        let mut switch_button = TheTraybarButton::new(TheId::named("Tilemap Selection Switch"));
        switch_button.set_text("Anim".to_string());
        switch_button
            .set_status_text("Switches between an anim based preview and multi tiles preview.");

        let mut icon_preview = TheIconView::new(TheId::named("Tilemap Selection Preview"));
        icon_preview.set_alpha_mode(false);
        icon_preview.limiter_mut().set_max_size(vec2i(100, 100));
        icon_preview.set_border_color(Some([100, 100, 100, 255]));

        vlayout.add_widget(Box::new(switch_button));
        vlayout.add_widget(Box::new(icon_preview));

        details_canvas.set_layout(vlayout);

        toolbar_canvas.set_layout(toolbar_hlayout);
        canvas.set_top(toolbar_canvas);
        canvas.set_right(details_canvas);

        canvas
    }

    pub fn set_tilemap(&mut self, tilemap: &tilemap::Tilemap, ui: &mut TheUI, _: &mut TheContext) {
        self.curr_tilemap_id = tilemap.id;
        if let Some(rgba_layout) = ui.get_rgba_layout("Tilemap Editor") {
            rgba_layout.set_buffer(tilemap.buffer.clone());
            rgba_layout.set_scroll_offset(tilemap.scroll_offset);
            if let Some(rgba_view) = rgba_layout.rgba_view_mut().as_rgba_view() {
                rgba_view.set_grid(Some(tilemap.grid_size));
                rgba_view.set_mode(TheRGBAViewMode::TileSelection);
            }
        }
    }

    pub fn set_tilemap_preview(&self, tile: TheRGBATile, ui: &mut TheUI) {
        if let Some(icon_view) = ui.get_icon_view("Tilemap Selection Preview") {
            icon_view.set_rgba_tile(tile);
        }
    }

    pub fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        _server: &mut Server,
        _server_ctx: &mut ServerContext,
    ) -> bool {
        let mut redraw = false;

        match event {
            TheEvent::DialogValueOnClose(role, name, uuid, value) => {
                if name == "Rename Tilemap" && *role == TheDialogButtonRole::Accept {
                    if let Some(tilemap) = project.get_tilemap(self.curr_tilemap_id) {
                        tilemap.name = value.describe();
                        ctx.ui.send(TheEvent::SetValue(*uuid, value.clone()));
                    }
                }
            }
            TheEvent::ContextMenuSelected(_widget_id, item_id) => {
                if item_id.name == "Rename Tilemap" {
                    if let Some(tilemap) = project.get_tilemap(self.curr_tilemap_id) {
                        open_text_dialog(
                            "Rename Tilemap",
                            "Tilemap Name",
                            tilemap.name.as_str(),
                            self.curr_tilemap_id,
                            ui,
                            ctx,
                        );
                    }
                }
            }
            TheEvent::TileSelectionChanged(id) => {
                if id.name == "Tilemap Editor View" {
                    if let Some(rgba_layout) = ui.get_rgba_layout("Tilemap Editor") {
                        if let Some(rgba_view) = rgba_layout.rgba_view_mut().as_rgba_view() {
                            if self.anim_preview {
                                let selection = rgba_view.selection_as_sequence();
                                let mut tile = TheRGBATile::default();
                                if let Some(tilemap) = project.get_tilemap(self.curr_tilemap_id) {
                                    tile.buffer = tilemap.buffer.extract_sequence(&selection);
                                }
                                self.set_tilemap_preview(tile, ui);
                            } else {
                                let mut tile = TheRGBATile::default();
                                let dim = rgba_view.selection_as_dim();

                                if let Some(tilemap) = project.get_tilemap(self.curr_tilemap_id) {
                                    let region = TheRGBARegion::new(
                                        dim.x as usize * tilemap.grid_size as usize,
                                        dim.y as usize * tilemap.grid_size as usize,
                                        dim.width as usize * tilemap.grid_size as usize,
                                        dim.height as usize * tilemap.grid_size as usize,
                                    );
                                    tile.buffer.push(tilemap.buffer.extract_region(&region));
                                }
                                self.set_tilemap_preview(tile, ui);
                            }
                        }
                    }
                }
            }
            TheEvent::StateChanged(id, state) => {
                if id.name == "Tilemap Selection Switch" && *state == TheWidgetState::Clicked {
                    if self.anim_preview {
                        self.anim_preview = false;
                        ui.set_widget_value(
                            "Tilemap Selection Switch",
                            ctx,
                            TheValue::Text("Multi".to_string()),
                        );
                    } else {
                        self.anim_preview = true;
                        ui.set_widget_value(
                            "Tilemap Selection Switch",
                            ctx,
                            TheValue::Text("Anim".to_string()),
                        );
                    }
                    ctx.ui.send(TheEvent::TileSelectionChanged(TheId::named(
                        "Tilemap Editor View",
                    )));
                    redraw = true;
                } else if id.name == "Tilemap Editor Clear" && *state == TheWidgetState::Clicked {
                    if let Some(editor) = ui
                        .canvas
                        .get_layout(Some(&"Tilemap Editor".to_string()), None)
                    {
                        if let Some(editor) = editor.as_rgba_layout() {
                            editor
                                .rgba_view_mut()
                                .as_rgba_view()
                                .unwrap()
                                .set_selection(FxHashSet::default());
                        }
                    }
                    self.set_tilemap_preview(TheRGBATile::default(), ui);
                }
            }
            TheEvent::ValueChanged(_id, _value) => {}
            _ => {}
        }
        redraw
    }
}
