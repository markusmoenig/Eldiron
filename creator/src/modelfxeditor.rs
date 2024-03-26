use crate::prelude::*;
use rayon::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ModelFXMode {
    Floor,
    Wall,
    Ceiling,
}

pub struct ModelFXEditor {
    pub mode: ModelFXMode,
    pub curr_timeline: TheTimeline,
    pub curr_collection: TheCollection,
    pub curr_marker: Option<TheTime>,

    pub fx_text_floor: FxHashMap<(i32, i32), String>,
    pub fx_text_wall: FxHashMap<(i32, i32), String>,

    pub curr_tile_id: TheId,
    pub curr_tile_pos: Vec2i,
}

#[allow(clippy::new_without_default)]
impl ModelFXEditor {
    pub fn new() -> Self {
        Self {
            mode: ModelFXMode::Floor,
            curr_timeline: TheTimeline::default(),
            curr_collection: TheCollection::default(),
            curr_marker: None,

            fx_text_floor: FxHashMap::default(),
            fx_text_wall: FxHashMap::default(),

            curr_tile_id: TheId::empty(),
            curr_tile_pos: Vec2i::zero(),
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
        let mut rgba_layout = TheRGBALayout::new(TheId::named("ModelFX Floor RGBA Layout"));
        rgba_layout.limiter_mut().set_max_width(130);

        if let Some(rgba_view) = rgba_layout.rgba_view_mut().as_rgba_view() {
            rgba_view.set_grid(Some(24));
            rgba_view.set_mode(TheRGBAViewMode::TilePicker);
            let mut c = WHITE;
            c[3] = 128;
            rgba_view.set_hover_color(Some(c));
            ctx.ui.send(TheEvent::Custom(
                TheId::named("Render Floor ModelFX Previews"),
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

        let mut text_layout = TheTextLayout::new(TheId::named("ModelFX Settings"));
        text_layout.set_fixed_text_width(90);
        //text_layout.limiter_mut().set_max_width(220);
        center_canvas.set_layout(text_layout);

        let mut center_texture_canvas = TheCanvas::default();
        let mut texture_layout = TheTextLayout::new(TheId::named("ModelFX Pattern Settings"));
        texture_layout.limiter_mut().set_max_width(190);
        texture_layout.set_background_color(Some(ListLayoutBackground));

        /*
        let mut color_picker = TheColorPicker::new(TheId::named("Pattern Color 1"));
        color_picker.limiter_mut().set_max_size(vec2i(90, 90));
        color_picker.set_color(TheColor::black().to_vec3f());
        texture_layout.add_pair(str!("Color 1"), Box::new(color_picker));

        let mut color_picker = TheColorPicker::new(TheId::named("Pattern Color 2"));
        color_picker.limiter_mut().set_max_size(vec2i(90, 90));
        color_picker.set_color(TheColor::white().to_vec3f());
        texture_layout.add_pair(str!("Color 2"), Box::new(color_picker));
        */

        center_texture_canvas.set_layout(texture_layout);

        center_canvas.set_right(center_texture_canvas);
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
                        if name == "Pattern" {
                            ctx.ui.send(TheEvent::TilePicked(
                                self.curr_tile_id.clone(),
                                self.curr_tile_pos,
                            ));
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
                if id.name == "Floor Selected" {
                    if let Some(stack) = ui.get_stack_layout("ModelFX Stack Layout") {
                        stack.set_index(0);
                        redraw = true;
                        ctx.ui.relayout = true;
                        self.mode = ModelFXMode::Floor;
                    }
                } else if id.name == "Wall Selected" {
                    if let Some(stack) = ui.get_stack_layout("ModelFX Stack Layout") {
                        stack.set_index(1);
                        redraw = true;
                        ctx.ui.relayout = true;
                        self.mode = ModelFXMode::Wall;
                    }
                } else if id.name == "Ceiling Selected" {
                    if let Some(stack) = ui.get_stack_layout("ModelFX Stack Layout") {
                        stack.set_index(2);
                        redraw = true;
                        ctx.ui.relayout = true;
                        self.mode = ModelFXMode::Ceiling;
                    }
                } else if id.name == "Render Floor ModelFX Previews" {
                    self.render_modelfx_floor_previews(ui, ctx);
                } else if id.name == "Render Wall ModelFX Previews" {
                    self.render_modelfx_wall_previews(ui, ctx);
                }
            }
            TheEvent::TilePicked(id, pos) => {
                if id.name == "ModelFX Floor RGBA Layout View"
                    || id.name == "ModelFX Wall RGBA Layout View"
                {
                    let fx_name = if self.mode == ModelFXMode::Floor {
                        self.fx_text_floor.get(&(pos.x, pos.y)).cloned()
                    } else {
                        self.fx_text_wall.get(&(pos.x, pos.y)).cloned()
                    };

                    self.curr_tile_id = id.clone();
                    self.curr_tile_pos = *pos;

                    if let Some(fx_name) = fx_name {
                        let c = self
                            .curr_timeline
                            .get_collection_at(&TheTime::default(), fx_name.to_string());

                        let mut collection: Option<TheCollection> = None;
                        let mut meta: Option<ModelFXMetaData> = None;
                        let mut unsupported: Vec<String> = vec![];

                        if self.mode == ModelFXMode::Floor {
                            let fx = ModelFXFloor::new_fx(&fx_name, c);
                            collection = Some(fx.collection_cloned());
                            if let Some(m) = fx.meta_data() {
                                meta = Some(m.clone());
                            }
                        } else if self.mode == ModelFXMode::Wall {
                            let fx = ModelFXWall::new_fx(&fx_name, c);
                            let cc = fx.collection_cloned();
                            unsupported = ModelFXWall::unsupported(&cc);
                            collection = Some(cc);
                            if let Some(m) = fx.meta_data() {
                                meta = Some(m.clone());
                            }
                        }

                        if let Some(collection) = collection {
                            if let Some(meta) = meta {
                                self.curr_collection = collection.clone();
                                if let Some(text_layout) = ui.get_text_layout("ModelFX Settings") {
                                    text_layout.clear();
                                    for (name, value) in &collection.keys {
                                        if unsupported.contains(name) {
                                            continue;
                                        }
                                        if let TheValue::FloatRange(value, range) = value {
                                            let mut slider = TheTextLineEdit::new(TheId::named(
                                                (":MODELFX: ".to_owned() + name).as_str(),
                                            ));
                                            slider.set_value(TheValue::Float(*value));
                                            //slider.set_default_value(TheValue::Float(0.0));
                                            slider.set_range(TheValue::RangeF32(range.clone()));
                                            slider.set_continuous(true);
                                            slider.set_status_text(
                                                meta.get_description(name).as_str(),
                                            );
                                            text_layout.add_pair(name.clone(), Box::new(slider));
                                        } else if let TheValue::IntRange(value, range) = value {
                                            let mut slider = TheTextLineEdit::new(TheId::named(
                                                (":MODELFX: ".to_owned() + name).as_str(),
                                            ));
                                            slider.set_value(TheValue::Int(*value));
                                            slider.set_range(TheValue::RangeI32(range.clone()));
                                            slider.set_status_text(
                                                meta.get_description(name).as_str(),
                                            );
                                            text_layout.add_pair(name.clone(), Box::new(slider));
                                        } else if let TheValue::TextList(index, list) = value {
                                            let mut dropdown = TheDropdownMenu::new(TheId::named(
                                                (":MODELFX: ".to_owned() + name).as_str(),
                                            ));
                                            for item in list {
                                                dropdown.add_option(item.clone());
                                            }
                                            dropdown.set_selected_index(*index);
                                            dropdown.set_status_text(
                                                meta.get_description(name).as_str(),
                                            );
                                            text_layout.add_pair(name.clone(), Box::new(dropdown));
                                        }
                                    }
                                    redraw = true;
                                    ctx.ui.relayout = true;
                                }
                                if let Some(text_layout) =
                                    ui.get_text_layout("ModelFX Pattern Settings")
                                {
                                    text_layout.clear();
                                    for (name, value) in &collection.keys {
                                        if unsupported.contains(name) {
                                            continue;
                                        }
                                        if let TheValue::ColorObject(color, _) = value {
                                            let mut color_picker =
                                                TheColorPicker::new(TheId::named(
                                                    (":MODELFX: ".to_owned() + name).as_str(),
                                                ));
                                            color_picker.limiter_mut().set_max_size(vec2i(90, 90));
                                            color_picker.set_color(color.to_vec3f());
                                            text_layout
                                                .add_pair(name.clone(), Box::new(color_picker));
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
                if self.mode == ModelFXMode::Floor {
                    if id.name == "ModelFX Floor RGBA Layout View" {
                        ctx.ui.send(TheEvent::SetStatusText(
                            id.clone(),
                            self.fx_text_floor
                                .get(&(pos.x, pos.y))
                                .unwrap_or(&"".to_string())
                                .to_string(),
                        ));
                    }
                } else if id.name == "ModelFX Wall RGBA Layout View" {
                    ctx.ui.send(TheEvent::SetStatusText(
                        id.clone(),
                        self.fx_text_wall
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

        let floor_fx = ModelFXFloor::parse_timeline(&time, &self.curr_timeline);
        let wall_fx = ModelFXWall::parse_timeline(&time, &self.curr_timeline);

        if let Some(render) = ui.get_render_view("ModelFX Preview") {
            let buffer = render.render_buffer_mut();

            let width = buffer.dim().width as usize;
            let height = buffer.dim().height as usize;

            let ro = vec3f(2.0, 2.0, 2.0);
            let rd = vec3f(0.0, 0.0, 0.0);

            let aa = 2;
            let aa_f = aa as f32;

            let camera = Camera::new(ro, rd, 160.0);

            buffer
                .pixels_mut()
                .par_rchunks_exact_mut(width * 4)
                .enumerate()
                .for_each(|(j, line)| {
                    for (i, pixel) in line.chunks_exact_mut(4).enumerate() {
                        let i = j * width + i;

                        let xx = (i % width) as f32;
                        let yy = (i / width) as f32;

                        let mut total = Vec4f::zero();

                        for m in 0..aa {
                            for n in 0..aa {
                                let camera_offset =
                                    vec2f(m as f32 / aa_f, n as f32 / aa_f) - vec2f(0.5, 0.5);

                                let mut color = vec4f(0.01, 0.01, 0.01, 1.0);

                                let ray = camera.create_ortho_ray(
                                    vec2f(xx / width as f32, 1.0 - yy / height as f32),
                                    vec2f(width as f32, height as f32),
                                    camera_offset,
                                );

                                let mut hit: Option<Hit> = None;

                                for fx in floor_fx.iter() {
                                    if let Some(h) = fx.hit(&ray) {
                                        if let Some(chit) = hit.clone() {
                                            if h.distance < chit.distance {
                                                hit = Some(h);
                                            }
                                        } else {
                                            hit = Some(h);
                                        }
                                    }
                                }

                                for fx in wall_fx.iter() {
                                    if let Some(h) = fx.hit(&ray) {
                                        if let Some(chit) = hit.clone() {
                                            if h.distance < chit.distance {
                                                hit = Some(h);
                                            }
                                        } else {
                                            hit = Some(h);
                                        }
                                    }
                                }

                                if let Some(hit) = hit {
                                    let c = dot(hit.normal, normalize(vec3f(1.0, 2.0, 3.0))) * 0.5
                                        + 0.5;
                                    color.x = c;
                                    color.y = c;
                                    color.z = c;
                                }

                                total += color;
                            }
                        }

                        let aa_aa = aa_f * aa_f;
                        total[0] /= aa_aa;
                        total[1] /= aa_aa;
                        total[2] /= aa_aa;
                        total[3] /= aa_aa;

                        pixel.copy_from_slice(&TheColor::from_vec4f(total).to_u8_array());
                    }
                });
        }
    }

    /// Render the floor previews.
    pub fn render_modelfx_floor_previews(&mut self, ui: &mut TheUI, ctx: &mut TheContext) {
        self.fx_text_floor.clear();

        if let Some(editor) = ui.get_rgba_layout("ModelFX Floor RGBA Layout") {
            let fx_array = ModelFXFloor::fx_array();

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

                    self.fx_text_floor.insert((x, y), fx.to_kind());

                    let mut rgba = TheRGBABuffer::new(TheDim::sized(grid, grid));

                    ModelFXFloor::render_preview(&mut rgba, vec![fx.clone()]);

                    buffer.copy_into(x * grid, y * grid, &rgba);
                    //buffer.copy_into(x * grid, y * grid, &tile.buffer[0].scaled(grid, grid));
                }

                rgba_view.set_buffer(buffer);
            }
            editor.relayout(ctx);
        }
    }

    /// Render the wall previews.
    pub fn render_modelfx_wall_previews(&mut self, ui: &mut TheUI, ctx: &mut TheContext) {
        self.fx_text_wall.clear();

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

                    self.fx_text_wall.insert((x, y), fx.to_kind());

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
