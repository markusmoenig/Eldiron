use crate::prelude::*;

pub struct TileFXEditor {
    pub curr_timeline: TheTimeline,
    pub curr_collection: TheCollection,
}

#[allow(clippy::new_without_default)]
impl TileFXEditor {
    pub fn new() -> Self {
        Self {
            curr_timeline: TheTimeline::default(),
            curr_collection: TheCollection::default(),
        }
    }

    /// Build the tile fx UI
    pub fn build(&self, ctx: &mut TheContext) -> TheCanvas {
        let mut canvas = TheCanvas::new();

        // Toolbar
        let mut toolbar_canvas = TheCanvas::default();
        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.limiter_mut().set_max_height(25);
        toolbar_hlayout.set_margin(vec4i(150, 2, 5, 3));

        let mut time_slider = TheTimeSlider::new(TheId::named("TileFX Timeline"));
        time_slider.set_status_text("The timeline for the tile based effects.");
        time_slider.limiter_mut().set_max_width(400);
        toolbar_hlayout.add_widget(Box::new(time_slider));

        let mut add_button = TheTraybarButton::new(TheId::named("TileFX Add"));
        //add_button.set_icon_name("icon_role_add".to_string());
        add_button.set_text(str!("Add FX"));
        add_button.set_status_text("Add the effect to the timeline.");
        let mut clear_button = TheTraybarButton::new(TheId::named("TileFX Clear"));
        //add_button.set_icon_name("icon_role_add".to_string());
        clear_button.set_text(str!("Clear FX"));
        clear_button.set_status_text("Clear the timeline.");

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

        let mut item = TheListItem::new(TheId::named("TileFX Brighness"));
        item.set_text(str!("Brightness"));
        list_layout.add_item(item, ctx);

        let mut item = TheListItem::new(TheId::named("TileFX Light Emitter"));
        item.set_text(str!("Light Emitter"));
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
        canvas.set_center(center_canvas);

        // Tile Preview

        let mut preview_canvas = TheCanvas::default();
        let mut tile_rgba = TheRGBAView::new(TheId::named("TileFX RGBA"));
        tile_rgba.set_mode(TheRGBAViewMode::TileSelection);
        tile_rgba.set_grid(Some(16));
        tile_rgba.set_buffer(TheRGBABuffer::new(TheDim::new(0, 0, 193, 193)));
        tile_rgba.limiter_mut().set_max_size(vec2i(193, 193));

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
            TheEvent::TileSelectionChanged(id) => {
                if id.name == "TileFX RGBA" {
                    if let Some(widget) = ui.get_widget("TileFX RGBA") {
                        if let Some(tile_rgba) = widget.as_rgba_view() {
                            let selection = tile_rgba.selection();

                            let mut lt = TheTileMask::default();
                            for s in &selection {
                                lt.add_pixel(vec2i(s.0, s.1), 12);
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
                    if let Some(id) = id.name.strip_prefix(":TILEFX: ") {
                        self.curr_collection.set(id, value.clone());
                    }
                }
            }
            TheEvent::StateChanged(id, state) => {
                if id.name == "TileFX Add" && *state == TheWidgetState::Clicked {
                    if let Some(time_slider) = ui.get_time_slider("TileFX Timeline") {
                        if let TheValue::Time(time) = time_slider.value() {
                            self.curr_timeline.add(time, self.curr_collection.clone());
                            time_slider.add_marker(time);
                            redraw = true;
                        }
                    }
                } else if id.name == "TileFX Clear" && *state == TheWidgetState::Clicked {
                    self.curr_timeline.clear();
                    if let Some(time_slider) = ui.get_time_slider("TileFX Timeline") {
                        time_slider.clear_marker();
                        redraw = true;
                    }
                } else if id.name.starts_with("TileFX") && *state == TheWidgetState::Selected {
                    let mut fx: Option<TileFX> = None;

                    if id.name == "TileFX Brighness" {
                        fx = Some(TileFX::new_fx("Brightness"));
                    } else if id.name == "TileFX Light Emitter" {
                        fx = Some(TileFX::new_fx("Light Emitter"));
                    }

                    if let Some(fx) = fx {
                        if let Some(text_layout) = ui.get_text_layout("TileFX Settings") {
                            if let Some(collection) = fx.collection() {
                                self.curr_collection = collection.clone();
                                text_layout.clear();
                                for (name, value) in &collection.keys {
                                    if let TheValue::FloatRange(value, range) = value {
                                        let mut slider = TheSlider::new(TheId::named(
                                            (":TILEFX: ".to_owned() + name).as_str(),
                                        ));
                                        slider.set_value(TheValue::Float(*value));
                                        slider.set_range(TheValue::RangeF32(range.clone()));
                                        text_layout.add_pair(name.clone(), Box::new(slider));
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
    pub fn set_timeline(&mut self, timeline: TheTimeline, ui: &mut TheUI) {
        self.curr_timeline = timeline;
        if let Some(time_slider) = ui.get_time_slider("TileFX Timeline") {
            time_slider.clear_marker();
            for time in self.curr_timeline.events.keys() {
                time_slider.add_marker(*time);
            }
        }
    }
}
