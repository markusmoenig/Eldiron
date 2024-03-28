use crate::prelude::*;

pub struct RegionFXEditor {
    pub curr_collection: TheCollection,
    pub curr_marker: Option<TheTime>,
}

#[allow(clippy::new_without_default)]
impl RegionFXEditor {
    pub fn new() -> Self {
        Self {
            curr_collection: TheCollection::default(),
            curr_marker: None,
        }
    }

    /// Build the UI
    pub fn build(&self, ctx: &mut TheContext) -> TheCanvas {
        let mut canvas = TheCanvas::new();

        // Toolbar
        let mut toolbar_canvas = TheCanvas::default();
        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.limiter_mut().set_max_height(25);
        toolbar_hlayout.set_margin(vec4i(100, 2, 5, 3));

        let mut time_slider = TheTimeSlider::new(TheId::named("RegionFX Timeline"));
        time_slider.set_status_text("The timeline for region based image effects.");
        time_slider.limiter_mut().set_max_width(400);
        toolbar_hlayout.add_widget(Box::new(time_slider));

        let mut add_button = TheTraybarButton::new(TheId::named("RegionFX Clear Marker"));
        //add_button.set_icon_name("icon_role_add".to_string());
        add_button.set_text(str!("Clear"));
        add_button.set_status_text("Clears the currently selected marker.");

        let mut clear_button = TheTraybarButton::new(TheId::named("RegionFX Clear All"));
        //add_button.set_icon_name("icon_role_add".to_string());
        clear_button.set_text(str!("Clear All"));
        clear_button.set_status_text("Clears all markers from the timeline.");

        toolbar_hlayout.add_widget(Box::new(add_button));
        toolbar_hlayout.add_widget(Box::new(clear_button));
        // toolbar_hlayout.set_reverse_index(Some(1));

        toolbar_canvas.set_layout(toolbar_hlayout);

        canvas.set_top(toolbar_canvas);

        // Left FX List

        let mut list_canvas = TheCanvas::default();
        let mut list_layout = TheListLayout::new(TheId::named("RegionFX List"));

        let mut item = TheListItem::new(TheId::named("RegionFX Camera"));
        item.set_text(str!("Camera"));
        list_layout.add_item(item, ctx);

        let mut item = TheListItem::new(TheId::named("RegionFX Distance / Fog"));
        item.set_text(str!("Distance / Fog"));
        list_layout.add_item(item, ctx);

        let mut item = TheListItem::new(TheId::named("RegionFX Saturation"));
        item.set_text(str!("Saturation"));
        list_layout.add_item(item, ctx);

        list_layout.limiter_mut().set_max_width(130);
        list_layout.select_first_item(ctx);
        list_canvas.set_layout(list_layout);

        canvas.set_left(list_canvas);

        // RegionFX Center

        let mut center_canvas = TheCanvas::default();

        let mut text_layout = TheTextLayout::new(TheId::named("RegionFX Settings"));
        text_layout.limiter_mut().set_max_width(300);
        text_layout.set_margin(vec4i(20, 15, 10, 10));
        center_canvas.set_layout(text_layout);

        let mut center_add_canvas = TheCanvas::default();
        let mut add_layout = TheTextLayout::new(TheId::named("RegionFX Add Settings"));
        add_layout.limiter_mut().set_max_width(350);
        add_layout.set_margin(vec4i(20, 15, 10, 10));
        add_layout.set_background_color(Some(ListLayoutBackground));
        center_add_canvas.set_layout(add_layout);

        center_canvas.set_right(center_add_canvas);
        canvas.set_center(center_canvas);

        canvas
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
            TheEvent::TimelineMarkerSelected(id, time) => {
                if id.name == "RegionFX Timeline" {
                    self.curr_marker = Some(*time);
                    redraw = true;
                }
            }
            TheEvent::ValueChanged(id, value) => {
                if id.name.starts_with(":REGIONFX:") {
                    if let Some(name) = id.name.strip_prefix(":REGIONFX: ") {
                        let mut value = value.clone();

                        // Correct values to their range variants if necessary as TheSlider strips them
                        // of the range
                        if let Some(TheValue::FloatRange(_, range)) = self.curr_collection.get(name)
                        {
                            if let Some(v) = value.to_f32() {
                                value = TheValue::FloatRange(v, range.clone());
                            }
                        } else if let Some(TheValue::IntRange(_, range)) =
                            self.curr_collection.get(name)
                        {
                            if let Some(v) = value.to_i32() {
                                value = TheValue::IntRange(v, range.clone());
                            }
                        } else if let Some(TheValue::TextList(_, list)) =
                            self.curr_collection.get(name)
                        {
                            if let Some(v) = value.to_i32() {
                                value = TheValue::TextList(v, list.clone());
                            }
                        }

                        self.curr_collection.set(name, value);

                        if let Some(time_slider) = ui.get_time_slider("RegionFX Timeline") {
                            if let TheValue::Time(time) = time_slider.value() {
                                if let Some(region) =
                                    project.get_region_mut(&server_ctx.curr_region)
                                {
                                    region.regionfx.add(time, self.curr_collection.clone());
                                    server.update_region(region);
                                    if let Some(names) =
                                        region.regionfx.get_collection_names_at(&time)
                                    {
                                        time_slider.add_marker(time, names);
                                    }
                                }
                                redraw = true;
                            }
                        }
                    }
                }
            }
            TheEvent::StateChanged(id, state) => {
                if id.name == "RegionFX Clear Marker" && *state == TheWidgetState::Clicked {
                    if let Some(time_slider) = ui.get_time_slider("RegionFX Timeline") {
                        if let Some(marker_time) = self.curr_marker {
                            if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                                region.regionfx.remove(&marker_time);
                                time_slider.remove_marker(marker_time);
                                self.curr_marker = None;
                                server.update_region(region);
                            }
                        }
                        redraw = true;
                    }
                } else if id.name == "RegionFX Clear All" && *state == TheWidgetState::Clicked {
                    if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                        region.regionfx.clear();
                        server.update_region(region);
                    }
                    if let Some(time_slider) = ui.get_time_slider("RegionFX Timeline") {
                        time_slider.clear_marker();
                        redraw = true;
                    }
                } else if id.name.starts_with("RegionFX ") && *state == TheWidgetState::Selected {
                    let fx_name = id.name.strip_prefix("RegionFX ").unwrap();
                    let mut c = None;

                    if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                        c = region
                            .regionfx
                            .get_collection_at(&TheTime::default(), fx_name.to_string());
                    }
                    let fx = Some(RegionFX::new_fx(fx_name, c));

                    if let Some(fx) = fx {
                        if let Some(collection) = fx.collection() {
                            self.curr_collection = collection.clone();
                            if let Some(text_layout) = ui.get_text_layout("RegionFX Settings") {
                                text_layout.clear();
                            }
                            if let Some(text_layout) = ui.get_text_layout("RegionFX Add Settings") {
                                text_layout.clear();
                            }
                            for (name, value) in &collection.keys {
                                if let TheValue::FloatRange(value, range) = value {
                                    let mut slider = TheTextLineEdit::new(TheId::named(
                                        (":REGIONFX: ".to_owned() + name).as_str(),
                                    ));
                                    slider.set_value(TheValue::Float(*value));
                                    //slider.set_default_value(TheValue::Float(0.0));
                                    slider.set_range(TheValue::RangeF32(range.clone()));
                                    slider.set_continuous(true);
                                    slider.set_status_text(fx.get_description(name).as_str());
                                    if let Some(meta) = fx.meta_data() {
                                        if meta.is_second_column(name) {
                                            if let Some(text_layout) =
                                                ui.get_text_layout("RegionFX Add Settings")
                                            {
                                                text_layout
                                                    .add_pair(name.clone(), Box::new(slider));
                                            }
                                        } else if let Some(text_layout) =
                                            ui.get_text_layout("RegionFX Settings")
                                        {
                                            text_layout.add_pair(name.clone(), Box::new(slider));
                                        }
                                    }
                                } else if let TheValue::IntRange(value, range) = value {
                                    let mut slider = TheTextLineEdit::new(TheId::named(
                                        (":REGIONFX: ".to_owned() + name).as_str(),
                                    ));
                                    slider.set_value(TheValue::Int(*value));
                                    slider.set_range(TheValue::RangeI32(range.clone()));
                                    slider.set_status_text(fx.get_description(name).as_str());
                                    if let Some(meta) = fx.meta_data() {
                                        if meta.is_second_column(name) {
                                            if let Some(text_layout) =
                                                ui.get_text_layout("RegionFX Add Settings")
                                            {
                                                text_layout
                                                    .add_pair(name.clone(), Box::new(slider));
                                            }
                                        } else if let Some(text_layout) =
                                            ui.get_text_layout("RegionFX Settings")
                                        {
                                            text_layout.add_pair(name.clone(), Box::new(slider));
                                        }
                                    }
                                } else if let TheValue::TextList(index, list) = value {
                                    let mut dropdown = TheDropdownMenu::new(TheId::named(
                                        (":REGIONFX: ".to_owned() + name).as_str(),
                                    ));
                                    for item in list {
                                        dropdown.add_option(item.clone());
                                    }
                                    dropdown.set_selected_index(*index);
                                    dropdown.set_status_text(fx.get_description(name).as_str());
                                    if let Some(meta) = fx.meta_data() {
                                        if meta.is_second_column(name) {
                                            if let Some(text_layout) =
                                                ui.get_text_layout("RegionFX Add Settings")
                                            {
                                                text_layout
                                                    .add_pair(name.clone(), Box::new(dropdown));
                                            }
                                        } else if let Some(text_layout) =
                                            ui.get_text_layout("RegionFX Settings")
                                        {
                                            text_layout.add_pair(name.clone(), Box::new(dropdown));
                                        }
                                    }
                                } else if let TheValue::Empty = value {
                                    let mut spacer = TheSpacer::new(TheId::empty());
                                    spacer.limiter_mut().set_max_size(vec2i(10, 5));
                                    if let Some(meta) = fx.meta_data() {
                                        if meta.is_second_column(name) {
                                            if let Some(text_layout) =
                                                ui.get_text_layout("RegionFX Add Settings")
                                            {
                                                text_layout
                                                    .add_pair(name.clone(), Box::new(spacer));
                                            }
                                        } else if let Some(text_layout) =
                                            ui.get_text_layout("RegionFX Settings")
                                        {
                                            text_layout.add_pair(name.clone(), Box::new(spacer));
                                        }
                                    }
                                }
                                redraw = true;
                                ctx.ui.relayout = true;
                            }
                            if let Some(vlayout) = ui.get_vlayout("RegionFX Color Settings") {
                                vlayout.clear();
                                for (name, value) in &collection.keys {
                                    if let TheValue::ColorObject(color) = value {
                                        let mut color_picker = TheColorPicker::new(TheId::named(
                                            (":REGIONFX: ".to_owned() + name).as_str(),
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
                    }
                }
            }
            _ => {}
        }

        redraw
    }

    /// Set the timeline from the picker
    pub fn set_region(&mut self, region: &Region, ui: &mut TheUI) {
        if let Some(time_slider) = ui.get_time_slider("RegionFX Timeline") {
            time_slider.clear_marker();
            for time in region.regionfx.events.keys() {
                if let Some(names) = region.regionfx.get_collection_names_at(time) {
                    time_slider.add_marker(*time, names);
                }
            }
        }
    }
}
