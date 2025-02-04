use crate::prelude::*;
use shared::prelude::*;

use crate::editor::{ACTIVEEDITOR, TILEDRAWER, UNDOMANAGER};

pub struct ModelEditor {
    pub palette_indices: FxHashMap<String, Vec<u16>>,
}

#[allow(clippy::new_without_default)]
impl ModelEditor {
    pub fn new() -> Self {
        Self {
            palette_indices: FxHashMap::default(),
        }
    }

    pub fn init_ui(
        &mut self,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        _project: &mut Project,
    ) -> TheCanvas {
        let mut center = TheCanvas::new();

        let mut model_editor = TheRGBALayout::new(TheId::named("Model Editor"));
        if let Some(rgba_view) = model_editor.rgba_view_mut().as_rgba_view() {
            rgba_view.set_mode(TheRGBAViewMode::TilePicker);
            rgba_view.set_dont_show_grid(true);

            //rgba_view.set_grid_color([255, 255, 255, 5]);
            //rgba_view.set_hover_color(Some([255, 255, 255, 100]));
            rgba_view.set_grid(Some(1));

            let buffer = TheRGBABuffer::new(TheDim::sized(400, 400));
            rgba_view.set_buffer(buffer);

            // Side Panel
            let mut side_panel = TheCanvas::new();
            let mut vlayout = TheVLayout::new(TheId::named("Model Panel Layout"));
            vlayout.set_background_color(Some(TheThemeColors::ListLayoutBackground));
            vlayout.limiter_mut().set_max_width(90);
            vlayout.set_margin(vec4i(0, 10, 0, 0));

            let mut seed_text = TheText::new(TheId::empty());
            seed_text.set_text("Seed Id".to_string());
            seed_text.set_text_color([200, 200, 200, 255]);
            vlayout.add_widget(Box::new(seed_text));

            let mut text = TheText::new(TheId::named("Seed Id"));
            text.set_text("-".to_string());
            text.set_text_color([200, 200, 200, 255]);
            vlayout.add_widget(Box::new(text));

            let mut hash_text = TheText::new(TheId::empty());
            hash_text.set_text("Hash Id".to_string());
            hash_text.set_text_color([200, 200, 200, 255]);
            vlayout.add_widget(Box::new(hash_text));

            let mut text = TheText::new(TheId::named("Pattern Id"));
            text.set_text("-".to_string());
            text.set_text_color([200, 200, 200, 255]);
            vlayout.add_widget(Box::new(text));

            side_panel.set_layout(vlayout);
            center.set_left(side_panel);
        }

        center.set_layout(model_editor);

        // Toolbar
        let mut top_toolbar = TheCanvas::new();
        top_toolbar.set_widget(TheTraybar::new(TheId::empty()));

        let mut toolbar_hlayout = TheHLayout::new(TheId::named("Model Tool Params"));
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(vec4i(10, 4, 5, 4));

        top_toolbar.set_layout(toolbar_hlayout);
        center.set_top(top_toolbar);

        center
    }

    pub fn build_node_ui(&mut self) -> TheCanvas {
        let mut center = TheCanvas::new();

        // Toolbar
        let mut top_toolbar = TheCanvas::new();
        top_toolbar.set_widget(TheTraybar::new(TheId::empty()));

        let mut toolbar_hlayout = TheHLayout::new(TheId::named("Model Bottom Toolbar"));
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(vec4i(10, 4, 5, 4));

        let mut material_nodes_button = TheTraybarButton::new(TheId::named("GeoFX Nodes"));
        material_nodes_button.set_text(str!("Material"));
        material_nodes_button.set_status_text("Material related nodes.");

        material_nodes_button.set_context_menu(Some(TheContextMenu {
            items: vec![TheContextMenuItem::new(
                "Material".to_string(),
                TheId::named("Material"),
            )],
            ..Default::default()
        }));

        let mut pattern_button = TheTraybarButton::new(TheId::named("GeoFX Nodes"));
        pattern_button.set_text(str!("Patterns"));
        pattern_button.set_status_text("The available patterns.");

        pattern_button.set_context_menu(Some(TheContextMenu {
            items: vec![
                TheContextMenuItem::new("Repeat".to_string(), TheId::named("Repeat")),
                TheContextMenuItem::new("Stack".to_string(), TheId::named("Stack")),
                TheContextMenuItem::new("Group".to_string(), TheId::named("Group")),
            ],
            ..Default::default()
        }));

        let mut shapes_button = TheTraybarButton::new(TheId::named("GeoFX Nodes"));
        shapes_button.set_text(str!("Shapes"));
        shapes_button.set_status_text("The available shapes.");

        shapes_button.set_context_menu(Some(TheContextMenu {
            items: vec![
                TheContextMenuItem::new("Box".to_string(), TheId::named("Box")),
                TheContextMenuItem::new("Disc".to_string(), TheId::named("Disc")),
            ],
            ..Default::default()
        }));

        let mut meta_button = TheTraybarButton::new(TheId::named("GeoFX Nodes"));
        meta_button.set_text(str!("Meta"));
        meta_button.set_status_text("The available meta nodes.");

        meta_button.set_context_menu(Some(TheContextMenu {
            items: vec![
                TheContextMenuItem::new("Meta Material".to_string(), TheId::named("Meta Material")),
                TheContextMenuItem::new("Meta Delete".to_string(), TheId::named("Meta Delete")),
            ],
            ..Default::default()
        }));

        toolbar_hlayout.add_widget(Box::new(pattern_button));
        toolbar_hlayout.add_widget(Box::new(shapes_button));
        toolbar_hlayout.add_widget(Box::new(material_nodes_button));
        toolbar_hlayout.add_widget(Box::new(meta_button));
        toolbar_hlayout.set_reverse_index(Some(4));

        top_toolbar.set_layout(toolbar_hlayout);
        center.set_top(top_toolbar);

        let node_view = TheNodeCanvasView::new(TheId::named("Model NodeCanvas"));
        center.set_widget(node_view);

        center
    }

    pub fn activated(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &ServerContext,
        update_nodes: bool,
    ) {
        let mut width = 100;
        let mut height = 100;

        if let Some(geo_obj_id) = server_ctx.curr_geo_object {
            if let Some(region) = project.get_region(&server_ctx.curr_region) {
                if let Some(geo_obj) = region.geometry.get(&geo_obj_id) {
                    width = (geo_obj.get_length() * 100.0) as usize;
                    height = (geo_obj.get_height() * 100.0) as usize;
                }
            }
        }

        if let Some(geo_obj_id) = server_ctx.curr_geo_object {
            if let Some(region) = project.get_region(&server_ctx.curr_region) {
                if let Some(editor) = ui.get_rgba_layout("Model Editor") {
                    if let Some(rgba_view) = editor.rgba_view_mut().as_rgba_view() {
                        let mut buffer =
                            TheRGBABuffer::new(TheDim::sized(width as i32, height as i32));
                        if let Some(ftctx) = region.compiled_geometry.get(&geo_obj_id) {
                            ftctx.render(width, height, buffer.pixels_mut());
                        }
                        rgba_view.set_buffer(buffer);
                        ctx.ui.relayout = true;
                        ctx.ui.redraw_all = true;
                    }
                }
            }
        }

        if update_nodes {
            if let Some(geo_obj_id) = server_ctx.curr_geo_object {
                if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                    if let Some(geo_obj) = region.geometry.get_mut(&geo_obj_id) {
                        let node_canvas = geo_obj.to_canvas();
                        ui.set_node_canvas("Model NodeCanvas", node_canvas);
                    }
                }
            }

            self.set_selected_geo_node_ui(server_ctx, project, ui, ctx, false);
        }
    }

    #[allow(clippy::too_many_arguments)]
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
            TheEvent::PaletteIndexChanged(_, index) => {
                if *ACTIVEEDITOR.lock().unwrap() == ActiveEditor::ModelEditor {
                    if let Some(geo_obj_id) = server_ctx.curr_geo_object {
                        let palette = project.palette.clone();
                        if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                            if let Some(geo_obj) = region.geometry.get_mut(&geo_obj_id) {
                                if let Some(selected_index) = geo_obj.selected_node {
                                    let prev = geo_obj.to_json();
                                    if geo_obj.nodes[selected_index].set_palette_index(*index) {
                                        let next = geo_obj.to_json();
                                        let geo_obj_id = geo_obj.id;

                                        let undo =
                                            RegionUndoAtom::GeoFXNodeEdit(geo_obj_id, prev, next);
                                        UNDOMANAGER
                                            .lock()
                                            .unwrap()
                                            .add_region_undo(&region.id, undo, ctx);

                                        region.compile_geo(
                                            geo_obj_id,
                                            &palette,
                                            &TILEDRAWER.lock().unwrap().tiles,
                                        );

                                        server.update_region(region);

                                        self.activated(ui, ctx, project, server_ctx, false);

                                        self.set_selected_geo_node_ui(
                                            server_ctx, project, ui, ctx, false,
                                        );
                                        redraw = true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            TheEvent::Custom(id, _) => {
                if id.name == "Update GeoFX Node" {
                    self.set_selected_geo_node_ui(server_ctx, project, ui, ctx, false);

                    let palette = project.palette.clone();
                    if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                        region.compile_geo(id.uuid, &palette, &TILEDRAWER.lock().unwrap().tiles);
                        self.activated(ui, ctx, project, server_ctx, false);
                    }
                }
            }
            TheEvent::ContextMenuSelected(id, item) => {
                //let prev = self.modelfx.to_json();
                #[allow(clippy::collapsible_if)]
                if id.name == "GeoFX Nodes" || id.name.is_empty() {
                    if let Some(geo_obj_id) = server_ctx.curr_geo_object {
                        if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                            if let Some(geo_obj) = region.geometry.get_mut(&geo_obj_id) {
                                let prev = geo_obj.to_json();
                                let mut node = GeoFXNode::new_from_name(item.name.clone());
                                node.position = vec2i(
                                    geo_obj.scroll_offset.x + 220,
                                    geo_obj.scroll_offset.y + 10,
                                );
                                geo_obj.nodes.push(node);
                                geo_obj.selected_node = Some(geo_obj.nodes.len() - 1);
                                geo_obj.update_area();
                                let undo = RegionUndoAtom::GeoFXAddNode(
                                    geo_obj.id,
                                    prev,
                                    geo_obj.to_json(),
                                );
                                UNDOMANAGER
                                    .lock()
                                    .unwrap()
                                    .add_region_undo(&region.id, undo, ctx);
                                let node_canvas = geo_obj.to_canvas();
                                ui.set_node_canvas("Model NodeCanvas", node_canvas);
                                self.set_selected_geo_node_ui(server_ctx, project, ui, ctx, false);
                            }
                        }
                    }
                    redraw = true;
                }
            }
            TheEvent::TileEditorHoverChanged(id, coord) => {
                if id.name == "Model Editor View" {
                    if let Some(geo_obj) = server_ctx.curr_geo_object {
                        if let Some(region) = project.get_region(&server_ctx.curr_region) {
                            if let Some(ftctx) = region.compiled_geometry.get(&geo_obj) {
                                let meta = ftctx.meta_data_at(coord.x, coord.y, 100, 100);
                                //println!("{:?}", meta);

                                if let Some(text) = ui.get_text("Seed Id") {
                                    if let Some(meta) = &meta {
                                        text.set_text(format!("{}", meta.seed_id));
                                    } else {
                                        text.set_text("-".to_string());
                                    }
                                }

                                if let Some(text) = ui.get_text("Pattern Id") {
                                    if let Some(meta) = &meta {
                                        text.set_text(format!("{}", meta.pattern_id));
                                    } else {
                                        text.set_text("-".to_string());
                                    }
                                }

                                if let Some(layout) = ui.get_layout("Model Panel Layout") {
                                    layout.relayout(ctx);
                                }
                                redraw = true;
                            }
                        }
                    }
                }
            }
            TheEvent::NodeSelectedIndexChanged(id, index) => {
                if id.name == "Model NodeCanvas" {
                    if let Some(geo_obj_id) = server_ctx.curr_geo_object {
                        if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                            if let Some(geo_obj) = region.geometry.get_mut(&geo_obj_id) {
                                geo_obj.selected_node = *index;
                            }
                        }
                    }
                    self.set_selected_geo_node_ui(server_ctx, project, ui, ctx, true);
                }
            }
            TheEvent::NodeDragged(id, index, position) => {
                if id.name == "Model NodeCanvas" {
                    if let Some(geo_obj_id) = server_ctx.curr_geo_object {
                        if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                            if let Some(geo_obj) = region.geometry.get_mut(&geo_obj_id) {
                                geo_obj.nodes[*index].position = *position;
                            }
                        }
                    }
                }
            }
            TheEvent::NodeConnectionAdded(id, connections)
            | TheEvent::NodeConnectionRemoved(id, connections) => {
                if id.name == "Model NodeCanvas" {
                    if let Some(geo_obj_id) = server_ctx.curr_geo_object {
                        let palette = project.palette.clone();
                        if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                            if let Some(geo_obj) = region.geometry.get_mut(&geo_obj_id) {
                                let prev = geo_obj.to_json();
                                geo_obj.connections.clone_from(connections);

                                // To update flexible outputs
                                let node_canvas = geo_obj.to_canvas();
                                ui.set_node_canvas("Model NodeCanvas", node_canvas);

                                let next = geo_obj.to_json();
                                let geo_obj_id = geo_obj.id;

                                let undo = RegionUndoAtom::GeoFXNodeEdit(geo_obj_id, prev, next);
                                UNDOMANAGER
                                    .lock()
                                    .unwrap()
                                    .add_region_undo(&region.id, undo, ctx);

                                region.compile_geo(
                                    geo_obj_id,
                                    &palette,
                                    &TILEDRAWER.lock().unwrap().tiles,
                                );

                                server.update_region(region);

                                self.activated(ui, ctx, project, server_ctx, false);
                                redraw = true;
                            }
                        }
                    }
                }
            }
            TheEvent::NodeDeleted(id, deleted_node_index, connections) => {
                if id.name == "Model NodeCanvas" {
                    if let Some(geo_obj_id) = server_ctx.curr_geo_object {
                        let palette = project.palette.clone();
                        if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                            if let Some(geo_obj) = region.geometry.get_mut(&geo_obj_id) {
                                let prev = geo_obj.to_json();
                                geo_obj.nodes.remove(*deleted_node_index);
                                geo_obj.connections.clone_from(connections);
                                geo_obj.selected_node = None;

                                // To update flexible outputs
                                let node_canvas = geo_obj.to_canvas();
                                ui.set_node_canvas("Model NodeCanvas", node_canvas);

                                let undo = RegionUndoAtom::GeoFXNodeEdit(
                                    geo_obj.id,
                                    prev,
                                    geo_obj.to_json(),
                                );
                                UNDOMANAGER
                                    .lock()
                                    .unwrap()
                                    .add_region_undo(&region.id, undo, ctx);

                                region.compile_geo(
                                    geo_obj_id,
                                    &palette,
                                    &TILEDRAWER.lock().unwrap().tiles,
                                );

                                server.update_region(region);
                                self.activated(ui, ctx, project, server_ctx, false);

                                redraw = true;
                            }
                        }
                    }
                }
            }
            TheEvent::NodeViewScrolled(id, offset) => {
                if id.name == "Model NodeCanvas" {
                    if let Some(geo_obj_id) = server_ctx.curr_geo_object {
                        if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                            if let Some(geo_obj) = region.geometry.get_mut(&geo_obj_id) {
                                geo_obj.scroll_offset = *offset;
                            }
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
                } else if id.name.starts_with(":GEOFX:") {
                    if let Some(name) = id.name.strip_prefix(":GEOFX: ") {
                        let mut value = value.clone();

                        if let Some(geo_obj_id) = server_ctx.curr_geo_object {
                            let palette = project.palette.clone();
                            if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                                if let Some(geo_obj) = region.geometry.get_mut(&geo_obj_id) {
                                    if let Some(selected_index) = geo_obj.selected_node {
                                        let mut old_tiles_to_render: Vec<Vec2i> = vec![];
                                        let mut new_tiles_to_render: Vec<Vec2i> = vec![];

                                        old_tiles_to_render.clone_from(&geo_obj.area);

                                        // Convert TextList back
                                        let coll = geo_obj.nodes[selected_index].collection();
                                        if let Some(TheValue::TextList(_, list)) = coll.get(name) {
                                            if let Some(v) = value.to_i32() {
                                                value = TheValue::TextList(v, list.clone());
                                            }
                                        }

                                        let prev = geo_obj.to_json();

                                        geo_obj.nodes[selected_index].set(name, value.clone());
                                        match &geo_obj.nodes[selected_index].role {
                                            GeoFXNodeRole::LeftWall
                                            | GeoFXNodeRole::BackWall
                                            | GeoFXNodeRole::RightWall
                                            | GeoFXNodeRole::FrontWall
                                            | GeoFXNodeRole::MiddleWallH
                                            | GeoFXNodeRole::MiddleWallV => {
                                                if name == "Length" || name == "Height" {
                                                    if let Some((node, _)) =
                                                        geo_obj.find_connected_input_node(0, 0)
                                                    {
                                                        let coll = geo_obj.nodes[node as usize]
                                                            .collection();
                                                        if coll.contains_key(name) {
                                                            geo_obj.nodes[node as usize]
                                                                .set(name, value);
                                                        }
                                                    }
                                                }
                                            }
                                            _ => {}
                                        }

                                        geo_obj.update_area();

                                        let next = geo_obj.to_json();

                                        new_tiles_to_render.clone_from(&geo_obj.area);
                                        let mut set: FxHashSet<Vec2i> = FxHashSet::default();
                                        set.extend(&old_tiles_to_render);
                                        set.extend(&new_tiles_to_render);

                                        let region_id = region.id;
                                        region.update_geometry_areas();

                                        region.compile_geo(
                                            geo_obj_id,
                                            &palette,
                                            &TILEDRAWER.lock().unwrap().tiles,
                                        );

                                        server.update_region(region);
                                        self.activated(ui, ctx, project, server_ctx, false);

                                        let undo =
                                            RegionUndoAtom::GeoFXNodeEdit(geo_obj_id, prev, next);
                                        UNDOMANAGER
                                            .lock()
                                            .unwrap()
                                            .add_region_undo(&region_id, undo, ctx);
                                    }
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

    pub fn set_selected_geo_node_ui(
        &mut self,
        server_ctx: &ServerContext,
        project: &mut Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        switch_to_nodes: bool,
    ) {
        self.palette_indices.clear();

        if let Some(geo_obj_id) = server_ctx.curr_geo_object {
            if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                if let Some(geo_obj) = region.geometry.get_mut(&geo_obj_id) {
                    if let Some(selected_index) = geo_obj.selected_node {
                        // Safeguard, not actually needed
                        if selected_index >= geo_obj.nodes.len() {
                            geo_obj.selected_node = None;
                            return;
                        }

                        let collection = geo_obj.nodes[selected_index].collection();

                        if let Some(text_layout) = ui.get_text_layout("Node Settings") {
                            text_layout.clear();

                            if switch_to_nodes {
                                ctx.ui.send(TheEvent::Custom(
                                    TheId::named("Show Node Settings"),
                                    TheValue::Text("Geometry Node".to_string()),
                                ));
                            }

                            for (name, value) in &collection.keys {
                                if let TheValue::Text(text) = value {
                                    let mut edit = TheTextLineEdit::new(TheId::named(
                                        (":GEOFX: ".to_owned() + name).as_str(),
                                    ));
                                    edit.set_value(TheValue::Text(text.clone()));
                                    text_layout.add_pair(name.clone(), Box::new(edit));
                                } else if let TheValue::FloatRange(value, range) = value {
                                    let mut slider = TheTextLineEdit::new(TheId::named(
                                        (":GEOFX: ".to_owned() + name).as_str(),
                                    ));
                                    slider.set_value(TheValue::Float(*value));
                                    //slider.set_default_value(TheValue::Float(0.0));
                                    slider.set_range(TheValue::RangeF32(range.clone()));
                                    //slider.set_continuous(true);
                                    text_layout.add_pair(name.clone(), Box::new(slider));
                                } else if let TheValue::IntRange(value, range) = value {
                                    let mut slider = TheTextLineEdit::new(TheId::named(
                                        (":GEOFX: ".to_owned() + name).as_str(),
                                    ));
                                    slider.set_value(TheValue::Int(*value));
                                    slider.set_range(TheValue::RangeI32(range.clone()));
                                    //slider.set_continuous(true);
                                    text_layout.add_pair(name.clone(), Box::new(slider));
                                } else if let TheValue::TextList(index, list) = value {
                                    let mut dropdown = TheDropdownMenu::new(TheId::named(
                                        (":GEOFX: ".to_owned() + name).as_str(),
                                    ));
                                    for item in list {
                                        dropdown.add_option(item.clone());
                                    }
                                    dropdown.set_selected_index(*index);
                                    text_layout.add_pair(name.clone(), Box::new(dropdown));
                                } else if let TheValue::PaletteIndex(index) = value {
                                    let name_id = ":GEOFX: ".to_owned() + name;
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
                    } else if let Some(text_layout) = ui.get_text_layout("Node Settings") {
                        text_layout.clear();
                    }
                    // PRERENDERTHREAD
                    //     .lock()
                    //     .unwrap()
                    //     .material_changed(material.clone());
                    // if let Some(region) = project.get_region(&server_ctx.curr_region) {
                    //     let area = region.get_material_area(material_id);
                    //     PRERENDERTHREAD.lock().unwrap().render_region(
                    //         region.clone(),
                    //         project.palette.clone(),
                    //         area,
                    //     );
                    // }
                }
            }
        } else if let Some(text_layout) = ui.get_text_layout("Node Settings") {
            text_layout.clear();
        }
    }
}
