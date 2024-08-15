use crate::prelude::*;

pub struct TileFXEditor {
    pub curr_timeline: TheTimeline,
    pub curr_collection: TheCollection,
    pub curr_marker: Option<TheTime>,
    pub preview_size: i32,

    pub object: TileFXObject,
    pub palette_indices: FxHashMap<String, Vec<u16>>,
}

#[allow(clippy::new_without_default)]
impl TileFXEditor {
    pub fn new() -> Self {
        Self {
            curr_timeline: TheTimeline::default(),
            curr_collection: TheCollection::default(),
            curr_marker: None,

            preview_size: 192,

            object: TileFXObject::default(),
            palette_indices: FxHashMap::default(),
        }
    }

    /// Build the tile fx UI
    pub fn build(&self, _ctx: &mut TheContext) -> TheCanvas {
        let mut canvas = TheCanvas::new();

        // Toolbar
        let mut toolbar_canvas = TheCanvas::default();
        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.limiter_mut().set_max_height(25);
        toolbar_hlayout.set_margin(vec4i(10, 2, 5, 3));

        let mut nodes_button = TheTraybarButton::new(TheId::named("TileFX Nodes"));
        //add_button.set_icon_name("icon_role_add".to_string());
        nodes_button.set_text(str!("Effect Nodes"));
        nodes_button.set_status_text("Available effect nodes.");
        nodes_button.set_context_menu(Some(TheContextMenu {
            items: vec![
                // TheContextMenuItem::new_submenu(
                //     "Effects".to_string(),
                //     TheId::named("Effect Nodes"),
                //     TheContextMenu {
                //         items: vec![TheContextMenuItem::new(
                //             "Brightness".to_string(),
                //             TheId::named("Brightness"),
                //         )],
                //         ..Default::default()
                //     },
                // ),
                TheContextMenuItem::new("Light Emitter".to_string(), TheId::named("Light Emitter")),
                TheContextMenuItem::new("Saturation".to_string(), TheId::named("Saturation")),
            ],
            ..Default::default()
        }));

        toolbar_hlayout.add_widget(Box::new(nodes_button));
        toolbar_hlayout.set_reverse_index(Some(1));

        toolbar_canvas.set_layout(toolbar_hlayout);

        canvas.set_top(toolbar_canvas);

        // Node Editor
        let mut node_canvas = TheCanvas::new();
        let node_view = TheNodeCanvasView::new(TheId::named("TileFX NodeCanvas"));
        node_canvas.set_widget(node_view);

        canvas.set_center(node_canvas);

        /*
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
                #[allow(clippy::collapsible_if)]
                if id.name == "TileFX Nodes" {
                    let mut node = TileFXNode::new_from_name(item.name.clone());
                    node.position = vec2i(
                        self.object.scroll_offset.x + 220,
                        self.object.scroll_offset.y + 10,
                    );
                    self.object.nodes.push(node);
                    self.object.selected_node = Some(self.object.nodes.len() - 1);

                    let node_canvas = self.object.to_canvas();
                    ui.set_node_canvas("TileFX NodeCanvas", node_canvas);
                    self.set_selected_node_ui(server_ctx, project, ui, ctx);

                    //if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                    /*
                    let prev = region.regionfx.clone();

                    let mut node = RegionFXNode::new_from_name(item.name.clone());
                    node.position = vec2i(
                        region.regionfx.scroll_offset.x + 220,
                        region.regionfx.scroll_offset.y + 10,
                    );
                    region.regionfx.nodes.push(node);
                    region.regionfx.selected_node = Some(region.regionfx.nodes.len() - 1);

                    let next = region.regionfx.clone();
                    let region_id = region.id;

                    let node_canvas = region.regionfx.to_canvas();
                    ui.set_node_canvas("RegionFX NodeCanvas", node_canvas);

                    self.set_selected_node_ui(server_ctx, project, ui, ctx);
                    let undo = RegionUndoAtom::RegionFXEdit(prev, next);
                    UNDOMANAGER
                        .lock()
                        .unwrap()
                        .add_region_undo(&region_id, undo, ctx);
                    //}
                    redraw = true;
                    */
                }
            }
            TheEvent::NodeSelectedIndexChanged(id, index) => {
                if id.name == "TileFX NodeCanvas" {
                    self.object.selected_node = *index;
                    self.set_selected_node_ui(server_ctx, project, ui, ctx);
                }
            }
            TheEvent::NodeDragged(id, index, position) => {
                if id.name == "TileFX NodeCanvas" {
                    self.object.nodes[*index].position = *position;
                }
            }
            TheEvent::NodeConnectionAdded(id, connections)
            | TheEvent::NodeConnectionRemoved(id, connections) => {
                if id.name == "TileFX NodeCanvas" {
                    if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                        self.object.connections.clone_from(connections);
                        redraw = true;
                        server.update_region(region);
                    }
                    /*
                    if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                        let prev = region.regionfx.clone();

                        region.regionfx.connections.clone_from(connections);

                        let next = region.regionfx.clone();
                        let region_id = region.id;

                        redraw = true;
                        server.update_region(region);

                        let undo = RegionUndoAtom::RegionFXEdit(prev, next);
                        UNDOMANAGER
                            .lock()
                            .unwrap()
                            .add_region_undo(&region_id, undo, ctx);

                        PRERENDERTHREAD
                            .lock()
                            .unwrap()
                            .render_region(region.clone(), None);
                    }*/
                }
            }
            TheEvent::NodeDeleted(id, deleted_node_index, connections) => {
                if id.name == "TileFX NodeCanvas" {
                    if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                        self.object.nodes.remove(*deleted_node_index);
                        self.object.connections.clone_from(connections);
                        self.object.selected_node = None;

                        redraw = true;
                        server.update_region(region);
                        /*
                        let prev = region.regionfx.clone();
                        region.regionfx.nodes.remove(*deleted_node_index);
                        region.regionfx.connections.clone_from(connections);
                        region.regionfx.selected_node = None;

                        let next = region.regionfx.clone();
                        let region_id = region.id;

                        let undo = RegionUndoAtom::RegionFXEdit(prev, next);
                        UNDOMANAGER
                            .lock()
                            .unwrap()
                            .add_region_undo(&region_id, undo, ctx);

                        redraw = true;
                        server.update_region(region);

                        PRERENDERTHREAD
                            .lock()
                            .unwrap()
                            .render_region(region.clone(), None);
                        */
                    }
                    self.set_selected_node_ui(server_ctx, project, ui, ctx);
                }
            }
            TheEvent::NodeViewScrolled(id, offset) => {
                if id.name == "TileFX NodeCanvas" {
                    self.object.scroll_offset = *offset;
                }
            }
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
                if id.name == "Palette Color Picker" {
                    let index = project.palette.current_index;
                    let mut widget_ids = Vec::new();
                    for (id, indices) in &self.palette_indices {
                        if indices.contains(&index) {
                            widget_ids.push(id.clone());
                        }
                    }

                    for widget_id in widget_ids {
                        if let Some(widget) = ui.get_widget(&widget_id) {
                            if let TheValue::ColorObject(color) = value {
                                widget.set_value(TheValue::ColorObject(color.clone()));
                            }
                        }
                    }
                } else if id.name.starts_with(":TILEFX:") {
                    if let Some(name) = id.name.strip_prefix(":TILEFX: ") {
                        let mut value = value.clone();

                        if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                            if let Some(selected_index) = self.object.selected_node {
                                // let prev = region.regionfx.clone();

                                // Convert TextList back
                                if let Some(TheValue::TextList(_, list)) =
                                    region.regionfx.nodes[selected_index].get(name)
                                {
                                    if let Some(v) = value.to_i32() {
                                        value = TheValue::TextList(v, list.clone());
                                    }
                                }

                                self.object.nodes[selected_index].set(name, value);

                                server.update_region(region);
                                //let next = material.to_json();

                                // if region.regionfx.nodes[selected_index].is_camera() {
                                //     PRERENDERTHREAD
                                //         .lock()
                                //         .unwrap()
                                //         .render_region(region.clone(), None);
                                // }

                                // let next = region.regionfx.clone();
                                // let region_id = region.id;

                                // let undo = RegionUndoAtom::RegionFXEdit(prev, next);
                                // UNDOMANAGER
                                //     .lock()
                                //     .unwrap()
                                //     .add_region_undo(&region_id, undo, ctx);
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
                } /*else if id.name.starts_with("TileFX ") && *state == TheWidgetState::Selected {
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
                                      } else if let TheValue::ColorObject(color) = value {
                                          let mut color_picker = TheColorPicker::new(TheId::named(
                                              (":TILEFX: ".to_owned() + name).as_str(),
                                          ));
                                          println!("here")
                                          color_picker.limiter_mut().set_max_size(vec2i(120, 120));
                                          color_picker.set_color(color.to_vec3f());
                                          text_layout.add_pair(name.clone(), Box::new(color_picker));
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
                  }*/
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

    pub fn set_selected_node_ui(
        &mut self,
        _server_ctx: &mut ServerContext,
        project: &mut Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) {
        self.palette_indices.clear();

        if let Some(selected_index) = self.object.selected_node {
            // Safeguard, not actually needed
            if selected_index >= self.object.nodes.len() {
                self.object.selected_node = None;
                return;
            }

            let collection = self.object.nodes[selected_index].collection();

            if let Some(text_layout) = ui.get_text_layout("Node Settings") {
                text_layout.clear();

                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Show Node Settings"),
                    TheValue::Text("RegionFX Node".to_string()),
                ));

                for (name, value) in &collection.keys {
                    if let TheValue::Text(text) = value {
                        let mut edit = TheTextLineEdit::new(TheId::named(
                            (":TILEFX: ".to_owned() + name).as_str(),
                        ));
                        edit.set_value(TheValue::Text(text.clone()));
                        text_layout.add_pair(name.clone(), Box::new(edit));
                    } else if let TheValue::FloatRange(value, range) = value {
                        let mut slider = TheTextLineEdit::new(TheId::named(
                            (":TILEFX: ".to_owned() + name).as_str(),
                        ));
                        slider.set_value(TheValue::Float(*value));
                        //slider.set_default_value(TheValue::Float(0.0));
                        slider.set_range(TheValue::RangeF32(range.clone()));
                        //slider.set_continuous(true);
                        text_layout.add_pair(name.clone(), Box::new(slider));
                    } else if let TheValue::IntRange(value, range) = value {
                        let mut slider = TheTextLineEdit::new(TheId::named(
                            (":TILEFX: ".to_owned() + name).as_str(),
                        ));
                        slider.set_value(TheValue::Int(*value));
                        slider.set_range(TheValue::RangeI32(range.clone()));
                        //slider.set_continuous(true);
                        text_layout.add_pair(name.clone(), Box::new(slider));
                    } else if let TheValue::TextList(index, list) = value {
                        let mut dropdown = TheDropdownMenu::new(TheId::named(
                            (":TILEFX: ".to_owned() + name).as_str(),
                        ));
                        for item in list {
                            dropdown.add_option(item.clone());
                        }
                        dropdown.set_selected_index(*index);
                        text_layout.add_pair(name.clone(), Box::new(dropdown));
                    } else if let TheValue::PaletteIndex(index) = value {
                        let name_id = ":TILEFX: ".to_owned() + name;
                        let mut color_picker = TheColorButton::new(TheId::named(name_id.as_str()));
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
                    } else if let TheValue::ColorObject(color) = value {
                        let mut color_picker = TheColorPicker::new(TheId::named(
                            (":TILEFX: ".to_owned() + name).as_str(),
                        ));
                        color_picker.limiter_mut().set_max_size(vec2i(120, 120));
                        color_picker.set_color(color.to_vec3f());
                        text_layout.add_pair(name.clone(), Box::new(color_picker));
                    }
                }
                ctx.ui.relayout = true;
            }
        } else if let Some(text_layout) = ui.get_text_layout("Node Settings") {
            text_layout.clear();
        }
    }
}
