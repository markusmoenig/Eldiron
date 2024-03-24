use crate::prelude::*;

pub struct ModelFXEditor {
    pub curr_timeline: TheTimeline,
    pub curr_collection: TheCollection,
    pub curr_marker: Option<TheTime>,

    pub fx_text: FxHashMap<(i32, i32), String>,
}

#[allow(clippy::new_without_default)]
impl ModelFXEditor {
    pub fn new() -> Self {
        Self {
            curr_timeline: TheTimeline::default(),
            curr_collection: TheCollection::default(),
            curr_marker: None,

            fx_text: FxHashMap::default(),
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

        // RGBA Stack

        let mut stack_canvas = TheCanvas::new();
        let mut stack_layout = TheStackLayout::new(TheId::named("ModelFX Stack Layout"));

        let mut rgba_canvas = TheCanvas::default();
        let mut rgba_layout = TheRGBALayout::new(TheId::named("ModelFX Ground RGBA Layout"));
        rgba_layout.limiter_mut().set_max_width(130);

        if let Some(rgba_view) = rgba_layout.rgba_view_mut().as_rgba_view() {
            rgba_view.set_grid(Some(24));
            rgba_view.set_mode(TheRGBAViewMode::TilePicker);
            let mut c = WHITE;
            c[3] = 128;
            rgba_view.set_hover_color(Some(c));
            ctx.ui.send(TheEvent::Custom(
                TheId::named("Render Ground ModelFX Previews"),
                TheValue::Empty,
            ));
        }
        rgba_canvas.set_layout(rgba_layout);
        stack_layout.add_canvas(rgba_canvas);
        stack_layout.limiter_mut().set_max_width(130);

        let mut rgba_canvas = TheCanvas::default();
        let mut rgba_layout = TheRGBALayout::new(TheId::named("ModelFX Wall RGBA Layout"));
        rgba_layout.limiter_mut().set_max_width(130);

        if let Some(rgba_view) = rgba_layout.rgba_view_mut().as_rgba_view() {
            rgba_view.set_grid(Some(24));
            rgba_view.set_mode(TheRGBAViewMode::TilePicker);
            let mut c = WHITE;
            c[3] = 128;
            rgba_view.set_hover_color(Some(c));
            ctx.ui.send(TheEvent::Custom(
                TheId::named("Render Wall ModelFX Previews"),
                TheValue::Empty,
            ));
        }
        rgba_canvas.set_layout(rgba_layout);
        stack_layout.add_canvas(rgba_canvas);

        let mut rgba_canvas = TheCanvas::default();
        let mut rgba_layout = TheRGBALayout::new(TheId::named("ModelFX Ceiling RGBA Layout"));
        rgba_layout.limiter_mut().set_max_width(130);

        if let Some(rgba_view) = rgba_layout.rgba_view_mut().as_rgba_view() {
            rgba_view.set_grid(Some(24));
            rgba_view.set_mode(TheRGBAViewMode::TilePicker);
            let mut c = WHITE;
            c[3] = 128;
            rgba_view.set_hover_color(Some(c));
            ctx.ui.send(TheEvent::Custom(
                TheId::named("Render Ceiling ModelFX Previews"),
                TheValue::Empty,
            ));
        }
        rgba_canvas.set_layout(rgba_layout);
        stack_layout.add_canvas(rgba_canvas);

        stack_canvas.set_layout(stack_layout);
        canvas.set_left(stack_canvas);

        // ModelFX Center

        let mut center_canvas = TheCanvas::default();

        let text_layout = TheTextLayout::new(TheId::named("ModelFX Settings"));
        //text_layout.limiter_mut().set_max_width(220);
        center_canvas.set_layout(text_layout);

        let mut center_material_canvas = TheCanvas::default();
        let mut material_layout = TheTextLayout::new(TheId::named("ModelFX Material Settings"));
        material_layout.limiter_mut().set_max_width(240);
        material_layout.set_background_color(Some(ListLayoutBackground));
        center_material_canvas.set_layout(material_layout);

        center_canvas.set_right(center_material_canvas);
        canvas.set_center(center_canvas);

        // Model Preview

        let mut preview_canvas = TheCanvas::default();
        let mut model_preview_render = TheRenderView::new(TheId::named("ModelFX Preview"));
        *model_preview_render.render_buffer_mut() = TheRGBABuffer::new(TheDim::sized(192, 192));
        model_preview_render
            .limiter_mut()
            .set_max_size(vec2i(192, 192));

        let mut vlayout = TheVLayout::new(TheId::empty());
        vlayout.limiter_mut().set_max_width(200);
        vlayout.add_widget(Box::new(model_preview_render));

        preview_canvas.set_layout(vlayout);
        canvas.set_right(preview_canvas);

        //

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
                        self.render_preview(ui, ctx);
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
                }
            }
            TheEvent::Custom(id, _) => {
                if id.name == "Ground Selected" {
                    if let Some(stack) = ui.get_stack_layout("ModelFX Stack Layout") {
                        stack.set_index(0);
                        redraw = true;
                        ctx.ui.relayout = true;
                    }
                } else if id.name == "Wall Selected" {
                    if let Some(stack) = ui.get_stack_layout("ModelFX Stack Layout") {
                        stack.set_index(1);
                        redraw = true;
                        ctx.ui.relayout = true;
                    }
                } else if id.name == "Ceiling Selected" {
                    if let Some(stack) = ui.get_stack_layout("ModelFX Stack Layout") {
                        stack.set_index(2);
                        redraw = true;
                        ctx.ui.relayout = true;
                    }
                } else if id.name == "Render Wall ModelFX Previews" {
                    self.render_modelfx_wall_previews(ui, ctx);
                }
            }
            TheEvent::TilePicked(id, pos) => {
                if id.name == "ModelFX Wall RGBA Layout View" {
                    if let Some(fx_name) = self.fx_text.get(&(pos.x, pos.y)) {
                        let c = self
                            .curr_timeline
                            .get_collection_at(&TheTime::default(), fx_name.to_string());
                        let fx = Some(ModelFXWall::new_fx(fx_name, c));

                        if let Some(fx) = fx {
                            if let Some(collection) = fx.collection() {
                                self.curr_collection = collection.clone();
                                if let Some(text_layout) = ui.get_text_layout("ModelFX Settings") {
                                    text_layout.clear();
                                    for (name, value) in &collection.keys {
                                        if let TheValue::FloatRange(value, range) = value {
                                            let mut slider = TheTextLineEdit::new(TheId::named(
                                                (":MODELFX: ".to_owned() + name).as_str(),
                                            ));
                                            slider.set_value(TheValue::Float(*value));
                                            //slider.set_default_value(TheValue::Float(0.0));
                                            slider.set_range(TheValue::RangeF32(range.clone()));
                                            slider.set_continuous(true);
                                            slider
                                                .set_status_text(fx.get_description(name).as_str());
                                            text_layout.add_pair(name.clone(), Box::new(slider));
                                        } else if let TheValue::IntRange(value, range) = value {
                                            let mut slider = TheTextLineEdit::new(TheId::named(
                                                (":MODELFX: ".to_owned() + name).as_str(),
                                            ));
                                            slider.set_value(TheValue::Int(*value));
                                            slider.set_range(TheValue::RangeI32(range.clone()));
                                            slider
                                                .set_status_text(fx.get_description(name).as_str());
                                            text_layout.add_pair(name.clone(), Box::new(slider));
                                        } else if let TheValue::TextList(index, list) = value {
                                            let mut dropdown = TheDropdownMenu::new(TheId::named(
                                                (":MODELFX: ".to_owned() + name).as_str(),
                                            ));
                                            for item in list {
                                                dropdown.add_option(item.clone());
                                            }
                                            dropdown.set_selected_index(*index);
                                            dropdown
                                                .set_status_text(fx.get_description(name).as_str());
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
                                            let mut color_picker =
                                                TheColorPicker::new(TheId::named(
                                                    (":MODELFX: ".to_owned() + name).as_str(),
                                                ));
                                            color_picker
                                                .limiter_mut()
                                                .set_max_size(vec2i(120, 120));
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
            }
            TheEvent::TileEditorHoverChanged(id, pos) => {
                if id.name == "ModelFX Wall RGBA Layout View" {
                    ctx.ui.send(TheEvent::SetStatusText(
                        id.clone(),
                        self.fx_text
                            .get(&(pos.x, pos.y))
                            .unwrap_or(&"".to_string())
                            .to_string(),
                    ));
                }
            }
            _ => {}
        }

        redraw
    }

    /// Render the preview.
    pub fn render_preview(&mut self, ui: &mut TheUI, _ctx: &mut TheContext) {
        let mut time = TheTime::default();
        if let Some(time_slider) = ui.get_time_slider("ModelFX Timeline") {
            if let TheValue::Time(t) = time_slider.value() {
                time = t;
            }
        }
        if let Some(render) = ui.get_render_view("ModelFX Preview") {
            let buffer = render.render_buffer_mut();
            let fx = ModelFXWall::parse_timeline(&time, &self.curr_timeline);
            ModelFXWall::render_preview(buffer, fx);
        }
    }

    /// Render the model previews.
    pub fn render_modelfx_wall_previews(&mut self, ui: &mut TheUI, ctx: &mut TheContext) {
        self.fx_text.clear();

        if let Some(editor) = ui.get_rgba_layout("ModelFX Wall RGBA Layout") {
            let fx_array = ModelFXWall::fx_array();

            let grid = 48;
            let width = grid * 2; //130 - 16; //editor.dim().width - 16;
            let height = fx_array.len() as i32 * 48 / 2;

            if let Some(rgba_view) = editor.rgba_view_mut().as_rgba_view() {
                rgba_view.set_grid(Some(grid));

                let tiles_per_row = width / grid;
                let lines = fx_array.len() as i32 / tiles_per_row + 1;

                let mut buffer =
                    TheRGBABuffer::new(TheDim::sized(width, max(lines * grid, height)));

                for (i, fx) in fx_array.iter().enumerate() {
                    let x = i as i32 % tiles_per_row;
                    let y = i as i32 / tiles_per_row;

                    self.fx_text.insert((x, y), fx.to_kind());

                    let mut rgba = TheRGBABuffer::new(TheDim::sized(grid, grid));

                    ModelFXWall::render_preview(&mut rgba, vec![fx.clone()]);

                    buffer.copy_into(x * grid, y * grid, &rgba);
                    //buffer.copy_into(x * grid, y * grid, &tile.buffer[0].scaled(grid, grid));
                }

                rgba_view.set_buffer(buffer);
            }
            editor.relayout(ctx);
        }
    }
}
