use crate::editor::TILEDRAWER;
use crate::prelude::*;
use rusterix::ValueContainer;
use theframework::prelude::*;

pub struct EffectPicker {
    pub id: String,

    pub effects_map: FxHashMap<(i32, i32), EffectWrapper>,
    pub effects_text: FxHashMap<(i32, i32), String>,

    pub filter: String,
    pub filter_role: u8,
    pub zoom: f32,

    pub effects: Vec<EffectWrapper>,

    pub curr_material: Option<Uuid>,
}

#[allow(clippy::new_without_default)]
impl EffectPicker {
    pub fn new(id: String) -> Self {
        let effects = vec![EffectWrapper::PointLight(ValueContainer::default())];

        Self {
            id,
            effects_map: FxHashMap::default(),
            effects_text: FxHashMap::default(),
            filter: "".to_string(),
            filter_role: 0,
            zoom: 1.0,

            effects,

            curr_material: None,
        }
    }

    /// Build the tile picker UI
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

        toolbar_hlayout.set_margin(Vec4::new(10, 1, 5, 1));
        toolbar_hlayout.set_padding(3);
        toolbar_hlayout.add_widget(Box::new(filter_text));
        let mut filter_edit = TheTextLineEdit::new(TheId::named(&self.make_id(" Filter Edit")));
        filter_edit.set_text("".to_string());
        filter_edit
            .limiter_mut()
            .set_max_size(Vec2::new(if minimal { 75 } else { 120 }, 18));
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

        let mut drop_down = TheDropdownMenu::new(TheId::named(&self.make_id(" Filter Role")));
        drop_down.add_option("All".to_string());
        for dir in TileRole::iterator() {
            drop_down.add_option(dir.to_string().to_string());
        }
        toolbar_hlayout.add_widget(Box::new(drop_down));

        if !minimal {
            let mut zoom = TheSlider::new(TheId::named(&self.make_id(" Zoom")));
            zoom.set_value(TheValue::Float(self.zoom));
            zoom.set_default_value(TheValue::Float(1.5));
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
            rgba_view.set_grid(Some(48));
            rgba_view.set_mode(TheRGBAViewMode::TilePicker);
            let c = [200, 200, 200, 200];
            rgba_view.set_hover_color(Some(c));
            rgba_view.set_selection_color(c);
        }

        /*
        // Details
        let mut details_canvas = TheCanvas::new();

        let mut vlayout = TheVLayout::new(TheId::named(" Tile Details Layout"));
        vlayout.set_margin(Vec4::new(5, 10, 5, 10));
        vlayout.set_alignment(TheHorizontalAlign::Left);
        vlayout.limiter_mut().set_max_width(150);

        let mut drop_down = TheDropdownMenu::new(TheId::named(&self.make_id(" Tile Role")));
        for dir in TileRole::iterator() {
            drop_down.add_option(dir.to_string().to_string());
        }
        drop_down.set_disabled(true);

        let mut blocking = TheDropdownMenu::new(TheId::named(&self.make_id(" Tile Blocking")));
        blocking.add_option("No".to_string());
        blocking.add_option("Yes".to_string());
        blocking.set_disabled(true);

        let mut tags = TheTextLineEdit::new(TheId::named(&self.make_id(" Tile Tags")));
        tags.limiter_mut().set_max_width(130);
        tags.set_disabled(true);

        let mut text = TheText::new(TheId::empty());
        text.set_text_size(12.0);
        text.set_text("Role".to_string());
        vlayout.add_widget(Box::new(text));
        vlayout.add_widget(Box::new(drop_down));

        let mut text = TheText::new(TheId::empty());
        text.set_text_size(12.0);
        text.set_text("Tags".to_string());
        vlayout.add_widget(Box::new(text));
        vlayout.add_widget(Box::new(tags));

        let mut text = TheText::new(TheId::empty());
        text.set_text_size(12.0);
        text.set_text("Blocking".to_string());
        vlayout.add_widget(Box::new(text));
        vlayout.add_widget(Box::new(blocking));

        let mut billboard_text = TheText::new(TheId::empty());
        billboard_text.set_text_size(12.0);
        billboard_text.set_text("Render in 3D".to_string());
        vlayout.add_widget(Box::new(billboard_text));

        let mut render_drop_down =
            TheDropdownMenu::new(TheId::named(&self.make_id(" Tile Billboard")));
        render_drop_down.add_option("As Cube".to_string());
        render_drop_down.add_option("As Billboard".to_string());
        vlayout.add_widget(Box::new(render_drop_down));

        details_canvas.set_layout(vlayout);

        //

        canvas.set_top(toolbar_canvas);
        */
        canvas.set_layout(rgba_layout);
        // canvas.set_right(details_canvas);

        canvas
    }

    /// Set the tiles for the picker.
    pub fn update_tiles(&mut self, _project: &Project, ui: &mut TheUI, ctx: &mut TheContext) {
        self.effects_map.clear();
        self.effects_text.clear();
        if let Some(editor) = ui.get_rgba_layout(&self.make_id(" RGBA Layout")) {
            let width = editor.dim().width - 16;
            let height = editor.dim().height - 16;

            if width == -16 {
                return;
            }

            if let Some(rgba_view) = editor.rgba_view_mut().as_rgba_view() {
                let grid = (48_f32 * self.zoom) as i32;
                rgba_view.set_grid(Some(grid));

                let mut filtered_tiles = vec![];

                for effect in &self.effects {
                    if effect.name().to_lowercase().contains(&self.filter)
                    //&& (self.filter_role == 0 || map.role == self.filter_role - 1)
                    {
                        filtered_tiles.push(effect.clone());
                    }
                }

                let tiles_per_row = width / grid;
                let lines = filtered_tiles.len() as i32 / tiles_per_row + 1;

                let mut buffer =
                    TheRGBABuffer::new(TheDim::sized(width, (lines * grid).max(height)));

                for (i, fx) in filtered_tiles.iter().enumerate() {
                    let x = i as i32 % tiles_per_row;
                    let y = i as i32 / tiles_per_row;

                    let mut rgba = TheRGBABuffer::from(
                        vec![0_u8; grid as usize * grid as usize * 4],
                        grid as u32,
                        grid as u32,
                    );

                    if let Some(icon) = ctx.ui.icon(&fx.icon()) {
                        rgba.copy_into(0, 0, icon);
                    }
                    self.effects_map.insert((x, y), fx.clone());
                    self.effects_text.insert((x, y), fx.name().clone());

                    buffer.copy_into(x * grid, y * grid, &rgba);
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
        server: &mut Server,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let mut redraw = false;

        match event {
            TheEvent::Resize => {
                self.update_tiles(project, ui, ctx);
            }
            TheEvent::TilePicked(id, pos) => {
                if id.name == self.make_id(" RGBA Layout View") {
                    if let Some(tile_id) = self.effects_map.get(&(pos.x, pos.y)) {
                        server_ctx.curr_effect = Some(tile_id.clone());
                        redraw = true;
                    }
                }
            }
            TheEvent::TileEditorHoverChanged(id, pos) => {
                if id.name == self.make_id(" RGBA Layout View") {
                    ctx.ui.send(TheEvent::SetStatusText(
                        id.clone(),
                        self.effects_text
                            .get(&(pos.x, pos.y))
                            .unwrap_or(&"".to_string())
                            .to_string(),
                    ));
                }
            }
            TheEvent::Custom(id, _value) => {
                if id.name == "Update Materialpicker" {
                    self.update_tiles(project, ui, ctx);
                }
            }
            // TheEvent::StateChanged(id, state) => {
            //     if id.name == self.make_id(" Tile Details") && *state == TheWidgetState::Clicked {
            //         if let Some(layout) = ui.get_layout(" Tile Details Layout") {
            //             if layout.limiter().get_max_width() == 0 {
            //                 layout.limiter_mut().set_max_width(150);
            //             } else {
            //                 layout.limiter_mut().set_max_width(0);
            //             }
            //             ctx.ui.relayout = true;
            //         }
            //         ctx.ui.send(TheEvent::Custom(
            //             TheId::named("Update Tilepicker"),
            //             TheValue::Empty,
            //         ));
            //     }
            //}
            TheEvent::ValueChanged(id, value) => {
                if id.name == self.make_id(" Tile Role") {
                    if let Some(tile_id) = self.curr_material {
                        if let Some(tile) = project.get_tile_mut(&tile_id) {
                            if let TheValue::Int(role) = value {
                                tile.role =
                                    TileRole::from_index(*role as u8).unwrap_or(TileRole::ManMade);
                            }
                        }
                    }
                } else if id.name == self.make_id(" Tile Tags") {
                    if let Some(tile_id) = self.curr_material {
                        if let Some(tile) = project.get_tile_mut(&tile_id) {
                            if let TheValue::Text(tags) = value {
                                tile.name.clone_from(tags);
                            }
                        }
                    }
                } else if id.name == self.make_id(" Tile Blocking") {
                    if let Some(tile_id) = self.curr_material {
                        if let Some(tile) = project.get_tile_mut(&tile_id) {
                            if let TheValue::Int(role) = value {
                                tile.blocking = *role == 1;
                            }
                        }
                    }
                } else if id.name == self.make_id(" Tile Billboard") {
                    if let Some(tile_id) = self.curr_material {
                        if let Some(tile) = project.get_tile_mut(&tile_id) {
                            if let TheValue::Int(billboard) = value {
                                tile.billboard = *billboard == 1;

                                let tiles = project.extract_tiles();
                                TILEDRAWER.lock().unwrap().set_tiles(tiles.clone());
                                server.update_tiles(tiles);
                            }
                        }
                    }
                } else if id.name == self.make_id(" Filter Edit") {
                    if let TheValue::Text(filter) = value {
                        self.filter = filter.to_lowercase();
                        self.update_tiles(project, ui, ctx);
                    }
                } else if id.name == self.make_id(" Filter Role") {
                    if let TheValue::Int(filter) = value {
                        self.filter_role = *filter as u8;
                        self.update_tiles(project, ui, ctx);
                    }
                } else if id.name == self.make_id(" Zoom") {
                    if let TheValue::Float(zoom) = value {
                        self.zoom = *zoom;
                        self.update_tiles(project, ui, ctx);
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
