use crate::prelude::*;

pub struct ModelFXEditor {
    pub curr_timeline: TheTimeline,
    pub curr_collection: TheCollection,
    pub curr_marker: Option<TheTime>,
}

#[allow(clippy::new_without_default)]
impl ModelFXEditor {
    pub fn new() -> Self {
        Self {
            curr_timeline: TheTimeline::default(),
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

        let mut time_slider = TheTimeSlider::new(TheId::named("ModelFX Timeline"));
        time_slider.set_status_text("The timeline for models.");
        time_slider.limiter_mut().set_max_width(400);
        toolbar_hlayout.add_widget(Box::new(time_slider));

        let mut add_button = TheTraybarButton::new(TheId::named("ModelFX Clear Marker"));
        //add_button.set_icon_name("icon_role_add".to_string());
        add_button.set_text(str!("Clear"));
        add_button.set_status_text("Clears the currently selected marker.");

        let mut clear_button = TheTraybarButton::new(TheId::named("ModelFX Clear All"));
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
        let mut list_layout = TheListLayout::new(TheId::named("ModelFX List"));

        let mut item = TheListItem::new(TheId::named("ModelFX Cube"));
        item.set_text(str!("Cube"));
        list_layout.add_item(item, ctx);

        let mut item = TheListItem::new(TheId::named("ModelFX Wall Horizontal"));
        item.set_text(str!("Wall Horizontal"));
        list_layout.add_item(item, ctx);

        let mut item = TheListItem::new(TheId::named("ModelFX Wall Vertical"));
        item.set_text(str!("Wall Vertical"));
        list_layout.add_item(item, ctx);

        list_layout.limiter_mut().set_max_width(130);
        list_layout.select_first_item(ctx);
        list_canvas.set_layout(list_layout);

        canvas.set_left(list_canvas);

        // ModelFX Center

        let mut center_canvas = TheCanvas::default();

        let mut text_layout = TheTextLayout::new(TheId::named("ModelFX Settings"));
        text_layout.limiter_mut().set_max_width(300);
        center_canvas.set_layout(text_layout);

        let mut center_color_canvas = TheCanvas::default();
        let mut color_layout = TheVLayout::new(TheId::named("ModelFX Color Settings"));
        color_layout.limiter_mut().set_max_width(140);
        color_layout.set_background_color(Some(ListLayoutBackground));
        center_color_canvas.set_layout(color_layout);

        center_canvas.set_right(center_color_canvas);
        canvas.set_center(center_canvas);

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
                if id.name == "ModelFX Timeline" {
                    self.curr_marker = Some(*time);
                    redraw = true;
                }
            }
            TheEvent::ValueChanged(id, value) => {
                if id.name.starts_with(":MODELFX:") {
                    if let Some(name) = id.name.strip_prefix(":MODELFX: ") {
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

                        if let Some(time_slider) = ui.get_time_slider("ModelFX Timeline") {
                            if let TheValue::Time(time) = time_slider.value() {
                                time_slider.add_marker(time, vec![]);
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
                if id.name == "ModelFX Clear Marker" && *state == TheWidgetState::Clicked {
                    if let Some(time_slider) = ui.get_time_slider("ModelFX Timeline") {
                        if let Some(marker_time) = self.curr_marker {
                            time_slider.remove_marker(marker_time);
                            self.curr_timeline.remove(&marker_time);
                            self.curr_marker = None;
                        }
                        redraw = true;
                    }
                } else if id.name == "ModelFX Clear All" && *state == TheWidgetState::Clicked {
                    self.curr_timeline.clear();
                    if let Some(time_slider) = ui.get_time_slider("ModelFX Timeline") {
                        time_slider.clear_marker();
                        redraw = true;
                    }
                } else if id.name.starts_with("ModelFX ") && *state == TheWidgetState::Selected {
                    let fx_name = id.name.strip_prefix("ModelFX ").unwrap();
                    let c = self
                        .curr_timeline
                        .get_collection_at(&TheTime::default(), fx_name.to_string());
                    let fx = Some(ModelFX::new_fx(fx_name, c));

                    if let Some(fx) = fx {
                        if let Some(collection) = fx.collection() {
                            self.curr_collection = collection.clone();
                            if let Some(text_layout) = ui.get_text_layout("ModelFX Settings") {
                                text_layout.clear();
                                for (name, value) in &collection.keys {
                                    if let TheValue::FloatRange(value, range) = value {
                                        let mut slider = TheSlider::new(TheId::named(
                                            (":MODELFX: ".to_owned() + name).as_str(),
                                        ));
                                        slider.set_value(TheValue::Float(*value));
                                        slider.set_default_value(TheValue::Float(0.0));
                                        slider.set_range(TheValue::RangeF32(range.clone()));
                                        slider.set_continuous(true);
                                        slider.set_status_text(fx.get_description(name).as_str());
                                        text_layout.add_pair(name.clone(), Box::new(slider));
                                    } else if let TheValue::IntRange(value, range) = value {
                                        let mut slider = TheSlider::new(TheId::named(
                                            (":MODELFX: ".to_owned() + name).as_str(),
                                        ));
                                        slider.set_value(TheValue::Int(*value));
                                        slider.set_range(TheValue::RangeI32(range.clone()));
                                        slider.set_status_text(fx.get_description(name).as_str());
                                        text_layout.add_pair(name.clone(), Box::new(slider));
                                    } else if let TheValue::TextList(index, list) = value {
                                        let mut dropdown = TheDropdownMenu::new(TheId::named(
                                            (":MODELFX: ".to_owned() + name).as_str(),
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
                            if let Some(vlayout) = ui.get_vlayout("ModelFX Color Settings") {
                                vlayout.clear();
                                for (name, value) in &collection.keys {
                                    if let TheValue::ColorObject(color, _) = value {
                                        let mut color_picker = TheColorPicker::new(TheId::named(
                                            (":MODELFX: ".to_owned() + name).as_str(),
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
}