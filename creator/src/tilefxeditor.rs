use crate::prelude::*;

pub struct TileFXEditor {
    pub curr_timeline: TheTimeline,
    pub curr_collection: TheCollection,
    pub curr_marker: Option<TheTime>,
    pub preview_size: i32,
}

#[allow(clippy::new_without_default)]
impl TileFXEditor {
    pub fn new() -> Self {
        Self {
            curr_timeline: TheTimeline::default(),
            curr_collection: TheCollection::default(),
            curr_marker: None,

            preview_size: 192,
        }
    }

    /// Build the tile fx UI
    pub fn build(&self, ctx: &mut TheContext) -> TheCanvas {
        let mut canvas = TheCanvas::new();

        // Toolbar
        let mut toolbar_canvas = TheCanvas::default();
        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.limiter_mut().set_max_height(25);
        toolbar_hlayout.set_margin(vec4i(120, 2, 5, 3));

        let mut time_slider = TheTimeSlider::new(TheId::named("TileFX Timeline"));
        time_slider.set_status_text("The timeline for the tile based effects.");
        time_slider.limiter_mut().set_max_width(400);
        toolbar_hlayout.add_widget(Box::new(time_slider));

        let mut add_button = TheTraybarButton::new(TheId::named("TileFX Clear Marker"));
        //add_button.set_icon_name("icon_role_add".to_string());
        add_button.set_text(str!("Clear"));
        add_button.set_status_text("Clears the currently selected marker.");

        let mut clear_button = TheTraybarButton::new(TheId::named("TileFX Clear"));
        //add_button.set_icon_name("icon_role_add".to_string());
        clear_button.set_text(str!("Clear All"));
        clear_button.set_status_text("Clears all markers from the timeline.");

        let mut clear_mask_button = TheTraybarButton::new(TheId::named("TileFX Clear Mask"));
        clear_mask_button.set_text(str!("Clear Mask"));
        clear_mask_button.set_status_text("Clear the pixel mask. If there are pixels selected the FX will only be applied to those pixels.");

        toolbar_hlayout.add_widget(Box::new(add_button));
        toolbar_hlayout.add_widget(Box::new(clear_button));
        toolbar_hlayout.add_widget(Box::new(clear_mask_button));
        toolbar_hlayout.set_reverse_index(Some(1));

        toolbar_canvas.set_layout(toolbar_hlayout);

        canvas.set_top(toolbar_canvas);

        // Left FX List

        let mut list_canvas = TheCanvas::default();
        let mut list_layout = TheListLayout::new(TheId::named("TileFX List"));

        let mut item = TheListItem::new(TheId::named("TileFX Brightness"));
        item.set_text(str!("Brightness"));
        list_layout.add_item(item, ctx);

        let mut item = TheListItem::new(TheId::named("TileFX Daylight"));
        item.set_text(str!("Daylight"));
        list_layout.add_item(item, ctx);

        let mut item = TheListItem::new(TheId::named("TileFX Light Emitter"));
        item.set_text(str!("Light Emitter"));
        list_layout.add_item(item, ctx);

        let mut item = TheListItem::new(TheId::named("TileFX Mirror"));
        item.set_text(str!("Mirror"));
        list_layout.add_item(item, ctx);

        list_layout.limiter_mut().set_max_width(130);
        list_layout.select_first_item(ctx);
        list_canvas.set_layout(list_layout);

        canvas.set_left(list_canvas);

        // Tile FX Center

        let mut center_canvas = TheCanvas::default();

        let mut text_layout = TheTextLayout::new(TheId::named("TileFX Settings"));
        text_layout.limiter_mut().set_max_width(300);
        center_canvas.set_layout(text_layout);

        let mut center_color_canvas = TheCanvas::default();
        let mut color_layout = TheVLayout::new(TheId::named("TileFX Color Settings"));
        color_layout.limiter_mut().set_max_width(140);
        color_layout.set_background_color(Some(ListLayoutBackground));
        center_color_canvas.set_layout(color_layout);

        center_canvas.set_right(center_color_canvas);
        canvas.set_center(center_canvas);

        // Tile Preview

        let mut preview_canvas = TheCanvas::default();
        let mut tile_rgba = TheRGBAView::new(TheId::named("TileFX RGBA"));
        tile_rgba.set_mode(TheRGBAViewMode::TileSelection);
        tile_rgba.set_grid(Some(self.preview_size / 24));
        tile_rgba.set_grid_color([40, 40, 40, 255]);
        tile_rgba.set_buffer(TheRGBABuffer::new(TheDim::new(
            0,
            0,
            self.preview_size,
            self.preview_size,
        )));
        tile_rgba
            .limiter_mut()
            .set_max_size(vec2i(self.preview_size, self.preview_size));

        let mut vlayout = TheVLayout::new(TheId::empty());
        vlayout.limiter_mut().set_max_width(200);
        vlayout.add_widget(Box::new(tile_rgba));

        preview_canvas.set_layout(vlayout);

        canvas.set_right(preview_canvas);

        canvas
    }

    pub fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        _project: &mut Project,
        _server: &mut Server,
        _server_ctx: &mut ServerContext,
    ) -> bool {
        let mut redraw = false;

        match event {
            TheEvent::TimelineMarkerSelected(id, time) => {
                if id.name == "TileFX Timeline" {
                    self.curr_marker = Some(*time);
                    redraw = true;
                }
            }
            TheEvent::TileSelectionChanged(id) => {
                if id.name == "TileFX RGBA" {
                    if let Some(widget) = ui.get_widget("TileFX RGBA") {
                        if let Some(tile_rgba) = widget.as_rgba_view() {
                            let selection = tile_rgba.selection();

                            let mut lt = TheTileMask::default();
                            for s in &selection {
                                lt.add_pixel(vec2i(s.0, s.1), true);
                            }
                            self.curr_collection.set("Mask", TheValue::TileMask(lt));

                            tile_rgba.set_needs_redraw(true);
                            redraw = true;
                        }
                    }
                }
            }
            TheEvent::ValueChanged(id, value) => {
                if id.name.starts_with(":TILEFX:") {
                    if let Some(name) = id.name.strip_prefix(":TILEFX: ") {
                        let mut value = value.clone();

                        // Correct values to their range variants if necessary as TheSlider strips them
                        // of the range
                        if let Some(TheValue::FloatRange(_, range)) = self.curr_collection.get(name)
                        {
                            value = TheValue::FloatRange(value.to_f32().unwrap(), range.clone());
                        } else if let Some(TheValue::IntRange(_, range)) =
                            self.curr_collection.get(name)
                        {
                            value = TheValue::IntRange(value.to_i32().unwrap(), range.clone());
                        } else if let Some(TheValue::TextList(_, list)) =
                            self.curr_collection.get(name)
                        {
                            value = TheValue::TextList(value.to_i32().unwrap(), list.clone());
                        }

                        self.curr_collection.set(name, value);

                        if let Some(time_slider) = ui.get_time_slider("TileFX Timeline") {
                            if let TheValue::Time(time) = time_slider.value() {
                                self.curr_timeline.add(time, self.curr_collection.clone());
                                if let Some(names) =
                                    self.curr_timeline.get_collection_names_at(&time)
                                {
                                    time_slider.add_marker(time, names);
                                }
                                redraw = true;
                            }
                        }
                    }
                }
            }
            TheEvent::StateChanged(id, state) => {
                if id.name == "TileFX Clear Mask" && *state == TheWidgetState::Clicked {
                    if let Some(widget) = ui.get_widget("TileFX RGBA") {
                        if let Some(tile_rgba) = widget.as_rgba_view() {
                            tile_rgba.set_selection(FxHashSet::default());
                            self.curr_collection
                                .set("Mask", TheValue::TileMask(TheTileMask::default()));
                            tile_rgba.set_needs_redraw(true);
                            redraw = true;
                        }
                    }
                } else if id.name == "TileFX Clear Marker" && *state == TheWidgetState::Clicked {
                    if let Some(time_slider) = ui.get_time_slider("TileFX Timeline") {
                        if let Some(marker_time) = self.curr_marker {
                            self.curr_timeline.remove(&marker_time);
                            time_slider.remove_marker(marker_time);
                            self.curr_marker = None;
                        }
                        redraw = true;
                    }
                } else if id.name == "TileFX Clear" && *state == TheWidgetState::Clicked {
                    self.curr_timeline.clear();
                    if let Some(time_slider) = ui.get_time_slider("TileFX Timeline") {
                        time_slider.clear_marker();
                        redraw = true;
                    }
                } else if id.name.starts_with("TileFX ") && *state == TheWidgetState::Selected {
                    let fx_name = id.name.strip_prefix("TileFX ").unwrap();
                    let c = self
                        .curr_timeline
                        .get_collection_at(&TheTime::default(), fx_name.to_string());

                    let fx = Some(TileFX::new_fx(fx_name, c));

                    if let Some(fx) = fx {
                        if let Some(collection) = fx.collection() {
                            self.curr_collection = collection.clone();
                            if let Some(text_layout) = ui.get_text_layout("TileFX Settings") {
                                text_layout.clear();
                                for (name, value) in &collection.keys {
                                    if let TheValue::FloatRange(value, range) = value {
                                        let mut slider = TheSlider::new(TheId::named(
                                            (":TILEFX: ".to_owned() + name).as_str(),
                                        ));
                                        slider.set_value(TheValue::Float(*value));
                                        slider.set_range(TheValue::RangeF32(range.clone()));
                                        slider.set_status_text(fx.get_description(name).as_str());
                                        text_layout.add_pair(name.clone(), Box::new(slider));
                                    } else if let TheValue::IntRange(value, range) = value {
                                        let mut slider = TheSlider::new(TheId::named(
                                            (":TILEFX: ".to_owned() + name).as_str(),
                                        ));
                                        slider.set_value(TheValue::Int(*value));
                                        slider.set_range(TheValue::RangeI32(range.clone()));
                                        slider.set_status_text(fx.get_description(name).as_str());
                                        text_layout.add_pair(name.clone(), Box::new(slider));
                                    } else if let TheValue::TextList(index, list) = value {
                                        let mut dropdown = TheDropdownMenu::new(TheId::named(
                                            (":TILEFX: ".to_owned() + name).as_str(),
                                        ));
                                        for item in list {
                                            dropdown.add_option(item.clone());
                                        }
                                        dropdown.set_selected_index(*index);
                                        dropdown.set_status_text(fx.get_description(name).as_str());
                                        text_layout.add_pair(name.clone(), Box::new(dropdown));
                                    }
                                }
                                redraw = true;
                                ctx.ui.relayout = true;
                            }
                            if let Some(vlayout) = ui.get_vlayout("TileFX Color Settings") {
                                vlayout.clear();
                                for (name, value) in &collection.keys {
                                    if let TheValue::ColorObject(color) = value {
                                        let mut color_picker = TheColorPicker::new(TheId::named(
                                            (":TILEFX: ".to_owned() + name).as_str(),
                                        ));
                                        color_picker.limiter_mut().set_max_size(vec2i(120, 120));
                                        color_picker.set_color(color.to_vec3f());
                                        vlayout.add_widget(Box::new(color_picker));
                                    }
                                }
                                redraw = true;
                                ctx.ui.relayout = true;
                            }
                        }

                        if let Some(TheValue::TileMask(mask)) = self.curr_collection.get("Mask") {
                            if let Some(widget) = ui.get_widget("TileFX RGBA") {
                                if let Some(tile_rgba) = widget.as_rgba_view() {
                                    let mut set = FxHashSet::default();

                                    for (index, value) in mask.pixels.iter() {
                                        if *value {
                                            set.insert((index.x, index.y));
                                        }
                                    }
                                    tile_rgba.set_selection(set);
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        redraw
    }

    /// Set the timeline from the picker
    pub fn set_timeline(&mut self, timeline: TheTimeline, ui: &mut TheUI) {
        self.curr_timeline = timeline;
        if let Some(time_slider) = ui.get_time_slider("TileFX Timeline") {
            time_slider.clear_marker();
            for time in self.curr_timeline.events.keys() {
                if let Some(names) = self.curr_timeline.get_collection_names_at(time) {
                    time_slider.add_marker(*time, names);
                }
            }
        }
    }
}
