use crate::prelude::*;

pub struct RegionFXEditor {
    pub curr_collection: TheCollection,
    pub curr_marker: Option<TheTime>,

    pub palette_indices: FxHashMap<String, Vec<u16>>,
}

#[allow(clippy::new_without_default)]
impl RegionFXEditor {
    pub fn new() -> Self {
        Self {
            curr_collection: TheCollection::default(),
            curr_marker: None,

            palette_indices: FxHashMap::default(),
        }
    }

    /// Build the UI
    pub fn build(&self, _ctx: &mut TheContext) -> TheCanvas {
        let mut canvas = TheCanvas::new();

        // Toolbar
        let mut toolbar_canvas = TheCanvas::default();
        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.limiter_mut().set_max_height(25);
        toolbar_hlayout.set_margin(vec4i(10, 2, 5, 3));

        let mut cameras_button = TheTraybarButton::new(TheId::named("RegionFX Camera Nodes"));
        //add_button.set_icon_name("icon_role_add".to_string());
        cameras_button.set_text(str!("Cameras"));
        cameras_button.set_status_text("Available cameras.");
        cameras_button.set_context_menu(Some(TheContextMenu {
            items: vec![
                TheContextMenuItem::new(
                    "Tilted Iso".to_string(),
                    TheId::named("Tilted Iso Camera"),
                ),
                TheContextMenuItem::new(
                    "Top Down Iso".to_string(),
                    TheId::named("Top Down Iso Camera"),
                ),
            ],
            ..Default::default()
        }));

        let mut nodes_button = TheTraybarButton::new(TheId::named("RegionFX Nodes"));
        //add_button.set_icon_name("icon_role_add".to_string());
        nodes_button.set_text(str!("Effect Nodes"));
        nodes_button.set_status_text("Available effect nodes.");
        nodes_button.set_context_menu(Some(TheContextMenu {
            items: vec![
                TheContextMenuItem::new_submenu(
                    "Effects".to_string(),
                    TheId::named("Effect Nodes"),
                    TheContextMenu {
                        items: vec![TheContextMenuItem::new(
                            "Brightness".to_string(),
                            TheId::named("Brightness"),
                        )],
                        ..Default::default()
                    },
                ),
                // TheContextMenuItem::new("Noise2D".to_string(), TheId::named("Noise2D")),
                // TheContextMenuItem::new("Noise3D".to_string(), TheId::named("Noise3D")),
            ],
            ..Default::default()
        }));

        toolbar_hlayout.add_widget(Box::new(cameras_button));
        toolbar_hlayout.add_widget(Box::new(nodes_button));
        toolbar_hlayout.set_reverse_index(Some(2));

        toolbar_canvas.set_layout(toolbar_hlayout);

        canvas.set_top(toolbar_canvas);

        // Node Editor
        let mut node_canvas = TheCanvas::new();
        let node_view = TheNodeCanvasView::new(TheId::named("RegionFX NodeCanvas"));
        node_canvas.set_widget(node_view);

        canvas.set_center(node_canvas);

        /*
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

        let mut item = TheListItem::new(TheId::named("RegionFX Renderer"));
        item.set_text(str!("Renderer"));
        list_layout.add_item(item, ctx);

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
        */

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
            TheEvent::ContextMenuSelected(id, item) => {
                //let prev = self.modelfx.to_json();
                #[allow(clippy::collapsible_if)]
                if id.name == "RegionFX Camera Nodes" {
                    if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                        // let prev = material.to_json();
                        // let material_id = material.id;
                        let mut node = RegionFXNode::new_from_name(item.name.clone());
                        node.position = vec2i(
                            region.render_settings.scroll_offset.x + 220,
                            region.render_settings.scroll_offset.y + 10,
                        );
                        region.render_settings.nodes.push(node);
                        region.render_settings.selected_node =
                            Some(region.render_settings.nodes.len() - 1);

                        let node_canvas = region.render_settings.to_canvas();
                        ui.set_node_canvas("RegionFX NodeCanvas", node_canvas);

                        self.set_selected_node_ui(server_ctx, project, ui, ctx);
                        // let undo =
                        //     MaterialFXUndoAtom::AddNode(material.id, prev, material.to_json());
                        // UNDOMANAGER.lock().unwrap().add_materialfx_undo(undo, ctx);
                        // let node_canvas = material.to_canvas(&project.palette);
                        // ui.set_node_canvas("MaterialFX NodeCanvas", node_canvas);
                        // self.set_material_tiles(ui, ctx, project, Some(material_id));
                        // self.set_selected_material_node_ui(server_ctx, project, ui, ctx);
                    }
                    redraw = true;
                }
            }
            TheEvent::NodeSelectedIndexChanged(id, index) => {
                if id.name == "RegionFX NodeCanvas" {
                    if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                        region.render_settings.selected_node = *index;
                    }
                    self.set_selected_node_ui(server_ctx, project, ui, ctx);
                }
            }
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

    pub fn set_selected_node_ui(
        &mut self,
        server_ctx: &mut ServerContext,
        project: &mut Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) {
        self.palette_indices.clear();

        if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
            if let Some(selected_index) = region.render_settings.selected_node {
                // Safeguard, not actually needed
                if selected_index >= region.render_settings.nodes.len() {
                    region.render_settings.selected_node = None;
                    return;
                }

                let collection = region.render_settings.nodes[selected_index].collection();

                if let Some(text_layout) = ui.get_text_layout("Node Settings") {
                    text_layout.clear();

                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Show Node Settings"),
                        TheValue::Text("RegionFX Node".to_string()),
                    ));

                    for (name, value) in &collection.keys {
                        if let TheValue::Text(text) = value {
                            let mut edit = TheTextLineEdit::new(TheId::named(
                                (":REGIONFX: ".to_owned() + name).as_str(),
                            ));
                            edit.set_value(TheValue::Text(text.clone()));
                            text_layout.add_pair(name.clone(), Box::new(edit));
                        } else if let TheValue::FloatRange(value, range) = value {
                            let mut slider = TheTextLineEdit::new(TheId::named(
                                (":REGIONFX: ".to_owned() + name).as_str(),
                            ));
                            slider.set_value(TheValue::Float(*value));
                            //slider.set_default_value(TheValue::Float(0.0));
                            slider.set_range(TheValue::RangeF32(range.clone()));
                            //slider.set_continuous(true);
                            text_layout.add_pair(name.clone(), Box::new(slider));
                        } else if let TheValue::IntRange(value, range) = value {
                            let mut slider = TheTextLineEdit::new(TheId::named(
                                (":REGIONFX: ".to_owned() + name).as_str(),
                            ));
                            slider.set_value(TheValue::Int(*value));
                            slider.set_range(TheValue::RangeI32(range.clone()));
                            //slider.set_continuous(true);
                            text_layout.add_pair(name.clone(), Box::new(slider));
                        } else if let TheValue::TextList(index, list) = value {
                            let mut dropdown = TheDropdownMenu::new(TheId::named(
                                (":REGIONFX: ".to_owned() + name).as_str(),
                            ));
                            for item in list {
                                dropdown.add_option(item.clone());
                            }
                            dropdown.set_selected_index(*index);
                            text_layout.add_pair(name.clone(), Box::new(dropdown));
                        } else if let TheValue::PaletteIndex(index) = value {
                            let name_id = ":MODELFX: ".to_owned() + name;
                            let mut color_picker =
                                TheColorButton::new(TheId::named(name_id.as_str()));
                            color_picker.limiter_mut().set_max_size(vec2i(80, 20));
                            if let Some(color) = &project.palette[*index as usize] {
                                color_picker.set_color(color.to_u8_array());
                            }

                            if let Some(indices) = self.palette_indices.get_mut(&name_id) {
                                indices.push(*index);
                            } else {
                                self.palette_indices
                                    .insert(name_id.to_string(), vec![*index]);
                            }
                            text_layout.add_pair(name.clone(), Box::new(color_picker));
                        }
                    }
                    ctx.ui.relayout = true;
                }
            }
        } else if let Some(text_layout) = ui.get_text_layout("Node Settings") {
            text_layout.clear();
        }
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
