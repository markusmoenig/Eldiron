use crate::editor::UNDOMANAGER;
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
        nodes_button.set_text(str!("Region FX"));
        nodes_button.set_status_text("Available region effect nodes.");
        nodes_button.set_context_menu(Some(TheContextMenu {
            items: vec![TheContextMenuItem::new(
                "Saturation".to_string(),
                TheId::named("Saturation"),
            )],
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
                #[allow(clippy::collapsible_if)]
                if id.name == "RegionFX Camera Nodes" || id.name == "RegionFX Nodes" {
                    if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
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
                    }
                    redraw = true;
                }
            }
            TheEvent::NodeSelectedIndexChanged(id, index) => {
                if id.name == "RegionFX NodeCanvas" {
                    if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                        region.regionfx.selected_node = *index;
                    }
                    self.set_selected_node_ui(server_ctx, project, ui, ctx);
                }
            }
            TheEvent::NodeDragged(id, index, position) => {
                if id.name == "RegionFX NodeCanvas" {
                    if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                        region.regionfx.nodes[*index].position = *position;
                    }
                }
            }
            TheEvent::NodeConnectionAdded(id, connections)
            | TheEvent::NodeConnectionRemoved(id, connections) => {
                if id.name == "RegionFX NodeCanvas" {
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
                    }
                }
            }
            TheEvent::NodeDeleted(id, deleted_node_index, connections) => {
                if id.name == "RegionFX NodeCanvas" {
                    if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
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
                    }
                    self.set_selected_node_ui(server_ctx, project, ui, ctx);
                }
            }
            TheEvent::NodeViewScrolled(id, offset) => {
                if id.name == "RegionFX NodeCanvas" {
                    if let Some(material_id) = server_ctx.curr_material_object {
                        if let Some(material) = project.materials.get_mut(&material_id) {
                            material.scroll_offset = *offset;
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
                } else if id.name.starts_with(":REGIONFX:") {
                    if let Some(name) = id.name.strip_prefix(":REGIONFX: ") {
                        let mut value = value.clone();

                        if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                            if let Some(selected_index) = region.regionfx.selected_node {
                                let prev = region.regionfx.clone();

                                // Convert TextList back
                                if let Some(TheValue::TextList(_, list)) =
                                    region.regionfx.nodes[selected_index].get(name)
                                {
                                    if let Some(v) = value.to_i32() {
                                        value = TheValue::TextList(v, list.clone());
                                    }
                                }

                                region.regionfx.nodes[selected_index].set(name, value);

                                server.update_region(region);
                                //let next = material.to_json();

                                let next = region.regionfx.clone();
                                let region_id = region.id;

                                let undo = RegionUndoAtom::RegionFXEdit(prev, next);
                                UNDOMANAGER
                                    .lock()
                                    .unwrap()
                                    .add_region_undo(&region_id, undo, ctx);
                            }
                        }
                    }
                }
            }
            TheEvent::Custom(id, _) => {
                if id.name == "Show RegionFX Node" {
                    self.set_selected_node_ui(server_ctx, project, ui, ctx);
                }
            }
            /*TheEvent::StateChanged(id, state) => {

            else if id.name.starts_with("RegionFX ") && *state == TheWidgetState::Selected {
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
                                // if let Some(meta) = fx.meta_data() {
                                //     if meta.is_second_column(name) {
                                //         if let Some(text_layout) =
                                //             ui.get_text_layout("RegionFX Add Settings")
                                //         {
                                //             text_layout
                                //                 .add_pair(name.clone(), Box::new(slider));
                                //         }
                                //     } else if let Some(text_layout) =
                                //         ui.get_text_layout("RegionFX Settings")
                                //     {
                                //         text_layout.add_pair(name.clone(), Box::new(slider));
                                //     }
                                // }
                            } else if let TheValue::IntRange(value, range) = value {
                                let mut slider = TheTextLineEdit::new(TheId::named(
                                    (":REGIONFX: ".to_owned() + name).as_str(),
                                ));
                                slider.set_value(TheValue::Int(*value));
                                slider.set_range(TheValue::RangeI32(range.clone()));
                                slider.set_status_text(fx.get_description(name).as_str());
                                // if let Some(meta) = fx.meta_data() {
                                //     if meta.is_second_column(name) {
                                //         if let Some(text_layout) =
                                //             ui.get_text_layout("RegionFX Add Settings")
                                //         {
                                //             text_layout
                                //                 .add_pair(name.clone(), Box::new(slider));
                                //         }
                                //     } else if let Some(text_layout) =
                                //         ui.get_text_layout("RegionFX Settings")
                                //     {
                                //         text_layout.add_pair(name.clone(), Box::new(slider));
                                //     }
                                // }
                            } else if let TheValue::TextList(index, list) = value {
                                let mut dropdown = TheDropdownMenu::new(TheId::named(
                                    (":REGIONFX: ".to_owned() + name).as_str(),
                                ));
                                for item in list {
                                    dropdown.add_option(item.clone());
                                }
                                dropdown.set_selected_index(*index);
                                dropdown.set_status_text(fx.get_description(name).as_str());
                                // if let Some(meta) = fx.meta_data() {
                                //     if meta.is_second_column(name) {
                                //         if let Some(text_layout) =
                                //             ui.get_text_layout("RegionFX Add Settings")
                                //         {
                                //             text_layout
                                //                 .add_pair(name.clone(), Box::new(dropdown));
                                //         }
                                //     } else if let Some(text_layout) =
                                //         ui.get_text_layout("RegionFX Settings")
                                //     {
                                //         text_layout.add_pair(name.clone(), Box::new(dropdown));
                                //     }
                                // }
                            } else if let TheValue::Empty = value {
                                let mut spacer = TheSpacer::new(TheId::empty());
                                spacer.limiter_mut().set_max_size(vec2i(10, 5));
                                // if let Some(meta) = fx.meta_data() {
                                //     if meta.is_second_column(name) {
                                //         if let Some(text_layout) =
                                //             ui.get_text_layout("RegionFX Add Settings")
                                //         {
                                //             text_layout
                                //                 .add_pair(name.clone(), Box::new(spacer));
                                //         }
                                //     } else if let Some(text_layout) =
                                //         ui.get_text_layout("RegionFX Settings")
                                //     {
                                //         text_layout.add_pair(name.clone(), Box::new(spacer));
                                //     }
                                // }
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
                    }*/
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
            if let Some(selected_index) = region.regionfx.selected_node {
                // Safeguard, not actually needed
                if selected_index >= region.regionfx.nodes.len() {
                    region.regionfx.selected_node = None;
                    return;
                }

                let collection = region.regionfx.nodes[selected_index].collection();

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
}
