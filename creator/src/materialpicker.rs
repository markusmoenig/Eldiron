use crate::prelude::*;
use rusterix::Value;

pub struct MaterialPicker {
    pub id: String,

    pub tile_ids: FxHashMap<(i32, i32), Uuid>,
    pub tile_text: FxHashMap<(i32, i32), String>,

    pub filter: String,
    pub filter_role: u8,
    pub zoom: f32,

    pub curr_material: Option<Uuid>,
}

#[allow(clippy::new_without_default)]
impl MaterialPicker {
    pub fn new(id: String) -> Self {
        Self {
            id,
            tile_ids: FxHashMap::default(),
            tile_text: FxHashMap::default(),
            filter: "".to_string(),
            filter_role: 0,
            zoom: 1.5,
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
            let mut c = WHITE;
            c[3] = 128;
            rgba_view.set_hover_color(Some(c));
        }

        // Details
        let mut details_canvas = TheCanvas::new();

        let mut vlayout = TheVLayout::new(TheId::named(" Tile Details Layout"));
        vlayout.set_margin(Vec4::new(5, 10, 5, 10));
        vlayout.set_alignment(TheHorizontalAlign::Left);
        vlayout.limiter_mut().set_max_width(150);

        /*
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
        */
        details_canvas.set_layout(vlayout);

        //

        canvas.set_top(toolbar_canvas);
        canvas.set_layout(rgba_layout);
        canvas.set_right(details_canvas);

        canvas
    }

    /// Set the tiles for the picker.
    pub fn update_materials(&mut self, project: &Project, ui: &mut TheUI, ctx: &mut TheContext) {
        self.tile_ids.clear();
        self.tile_text.clear();
        if let Some(editor) = ui.get_rgba_layout(&self.make_id(" RGBA Layout")) {
            //println!("{}", editor.dim().width);
            let width = editor.dim().width - 16;
            let height = editor.dim().height - 16;

            if width == -16 {
                return;
            }

            if let Some(rgba_view) = editor.rgba_view_mut().as_rgba_view() {
                let grid = (48_f32 * self.zoom) as i32;
                rgba_view.set_grid(Some(grid));

                let mut filtered_tiles = vec![];

                for (_, map) in &project.materials {
                    if map.name.to_lowercase().contains(&self.filter)
                    //&& (self.filter_role == 0 || map.role == self.filter_role - 1)
                    {
                        filtered_tiles.push(map.clone());
                    }
                }

                let tiles_per_row = width / grid;
                let lines = filtered_tiles.len() as i32 / tiles_per_row + 1;

                let mut buffer =
                    TheRGBABuffer::new(TheDim::sized(width, (lines * grid).max(height)));

                for (i, map) in filtered_tiles.iter().enumerate() {
                    let x = i as i32 % tiles_per_row;
                    let y = i as i32 / tiles_per_row;

                    self.tile_ids.insert((x, y), map.id);
                    self.tile_text.insert((x, y), map.name.clone());
                    if let Some(Value::Texture(texture)) = map.properties.get("material") {
                        let resized = texture.resized(grid as usize, grid as usize);
                        let rgba = TheRGBABuffer::from(
                            resized.data.clone(),
                            resized.width as u32,
                            resized.height as u32,
                        );
                        buffer.copy_into(x * grid, y * grid, &rgba);
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
        server_ctx: &mut ServerContext,
    ) -> bool {
        let mut redraw = false;

        match event {
            TheEvent::Resize => {
                self.update_materials(project, ui, ctx);
            }
            TheEvent::TilePicked(id, pos) => {
                if id.name == self.make_id(" RGBA Layout View") {
                    if let Some(tile_id) = self.tile_ids.get(&(pos.x, pos.y)) {
                        server_ctx.curr_material_id = Some(*tile_id);
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Material Picked"),
                            TheValue::Id(*tile_id),
                        ));
                        self.curr_material = Some(*tile_id);
                        self.apply_material(ui, ctx, project);
                        redraw = true;
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
                    self.update_materials(project, ui, ctx);
                }
            }
            TheEvent::Custom(id, _value) => {
                if id.name == "Update Materialpicker" {
                    self.update_materials(project, ui, ctx);
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
                } else
                /*  if id.name == self.make_id(" Tile Billboard") {
                    if let Some(tile_id) = self.curr_material {
                        if let Some(tile) = project.get_tile_mut(&tile_id) {
                            if let TheValue::Int(billboard) = value {
                                tile.billboard = *billboard == 1;
                            }
                        }
                    }
                } else*/
                if id.name == self.make_id(" Filter Edit") {
                    if let TheValue::Text(filter) = value {
                        self.filter = filter.to_lowercase();
                        self.update_materials(project, ui, ctx);
                    }
                } else if id.name == self.make_id(" Filter Role") {
                    if let TheValue::Int(filter) = value {
                        self.filter_role = *filter as u8;
                        self.update_materials(project, ui, ctx);
                    }
                } else if id.name == self.make_id(" Zoom") {
                    if let TheValue::Float(zoom) = value {
                        self.zoom = *zoom;
                        self.update_materials(project, ui, ctx);
                    }
                }
            }
            _ => {}
        }
        redraw
    }

    fn apply_material(&mut self, ui: &mut TheUI, _: &mut TheContext, project: &mut Project) {
        let mut tile: Option<&Tile> = None;

        if let Some(id) = &self.curr_material {
            tile = project.get_tile(id);
        }

        if let Some(widget) = ui.get_text_line_edit(&self.make_id(" Tile Tags")) {
            if let Some(tile) = tile {
                widget.set_text(tile.name.clone());
                widget.set_disabled(false);
            } else {
                widget.set_disabled(true);
            }
        }

        if let Some(widget) = ui.get_drop_down_menu(&self.make_id(" Tile Role")) {
            if let Some(tile) = tile {
                widget.set_selected_index(tile.role as i32);
                widget.set_disabled(false);
            } else {
                widget.set_disabled(true);
            }
        }

        if let Some(widget) = ui.get_drop_down_menu(&self.make_id(" Tile Blocking")) {
            if let Some(tile) = tile {
                widget.set_selected_index(if tile.blocking { 1 } else { 0 });
                widget.set_disabled(false);
            } else {
                widget.set_disabled(true);
            }
        }

        if let Some(widget) = ui.get_drop_down_menu(&self.make_id(" Tile Rendermode")) {
            if let Some(tile) = tile {
                widget.set_selected_index(tile.render_mode as i32);
                widget.set_disabled(false);
            } else {
                widget.set_disabled(true);
            }
        }
    }

    ///  Create an id.
    fn make_id(&self, id: &str) -> String {
        self.id.to_owned() + id
    }
}
