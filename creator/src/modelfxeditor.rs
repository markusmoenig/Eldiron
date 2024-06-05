use crate::editor::{PRERENDERTHREAD, SIDEBARMODE, UNDOMANAGER};
use crate::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ModelFXMode {
    Floor,
    Wall,
    Ceiling,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EditingMode {
    Geometry,
    Material,
    MaterialNodes,
}

pub struct ModelFXEditor {
    pub mode: ModelFXMode,
    pub geos: FxHashMap<(i32, i32), GeoFXNode>,
    pub materials: FxHashMap<(i32, i32), Uuid>,

    pub modelfx: ModelFX,
    pub editing_mode: EditingMode,

    pub current_material: Option<Uuid>,

    pub palette_indices: FxHashMap<String, Vec<u16>>,
}

#[allow(clippy::new_without_default)]
impl ModelFXEditor {
    pub fn new() -> Self {
        Self {
            mode: ModelFXMode::Floor,
            geos: FxHashMap::default(),
            materials: FxHashMap::default(),

            modelfx: ModelFX::default(),
            editing_mode: EditingMode::Geometry,

            current_material: None,

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

        // let mut clear_button: TheTraybarButton =
        //     TheTraybarButton::new(TheId::named("ModelFX Clear"));
        // clear_button.set_icon_name("trash".to_string());
        // clear_button.set_status_text("Clears the model, deleting all nodes.");

        // let mut move_button: TheTraybarButton = TheTraybarButton::new(TheId::named("ModelFX Move"));
        // move_button.set_icon_name("move".to_string());
        // move_button.set_status_text("Moves the model to the library.");

        let mut gb = TheGroupButton::new(TheId::named("ModelFX Mode Group"));
        gb.add_text_status(
            str!("Geometry"),
            str!("Nodes which model geometry like floors, walls and ceilings."),
        );
        gb.add_text_status(
            str!("Materials"),
            str!("Materials which can be applied to geometry nodes."),
        );
        gb.add_text_status(str!("Editor"), str!("Edit the current material."));
        gb.set_item_width(75);

        let mut floors_button = TheTraybarButton::new(TheId::named("ModelFX Nodes Floor"));
        //add_button.set_icon_name("icon_role_add".to_string());
        floors_button.set_text(str!("Floor & Furniture"));
        floors_button.set_status_text("Nodes which model floors and furniture like tables.");

        floors_button.set_context_menu(Some(TheContextMenu {
            items: vec![TheContextMenuItem::new(
                "Floor".to_string(),
                TheId::named("Floor"),
            )],
            ..Default::default()
        }));

        let mut walls_button = TheTraybarButton::new(TheId::named("ModelFX Nodes Wall"));
        //add_button.set_icon_name("icon_role_add".to_string());
        walls_button.set_text(str!("Wall & Components"));
        walls_button.set_status_text(
            "Nodes which model walls and components like windows, doors and decoration.",
        );

        walls_button.set_context_menu(Some(TheContextMenu {
            items: vec![
                TheContextMenuItem::new_submenu(
                    "Shapes".to_string(),
                    TheId::named("ModelFX Nodes Patterns"),
                    TheContextMenu {
                        items: vec![TheContextMenuItem::new(
                            "Capsule".to_string(),
                            TheId::named("Capsule"),
                        )],
                        ..Default::default()
                    },
                ),
                TheContextMenuItem::new(
                    "Wall Horizontal".to_string(),
                    TheId::named("Wall Horizontal"),
                ),
                TheContextMenuItem::new("Wall Vertical".to_string(), TheId::named("Wall Vertical")),
            ],
            ..Default::default()
        }));

        let mut material_button = TheTraybarButton::new(TheId::named("ModelFX Nodes Material"));
        //add_button.set_icon_name("icon_role_add".to_string());
        material_button.set_text(str!("Material Related"));
        material_button.set_status_text(
            "Nodes which model walls and components like windows, doors and decoration.",
        );

        material_button.set_context_menu(Some(TheContextMenu {
            items: vec![
                TheContextMenuItem::new_submenu(
                    "Patterns".to_string(),
                    TheId::named("ModelFX Nodes Patterns"),
                    TheContextMenu {
                        items: vec![
                            TheContextMenuItem::new(
                                "Bricks & Tiles".to_string(),
                                TheId::named("Bricks"),
                            ),
                            TheContextMenuItem::new(
                                "Steepness".to_string(),
                                TheId::named("Steepness"),
                            ),
                            TheContextMenuItem::new(
                                "Subdivide".to_string(),
                                TheId::named("Subdivide"),
                            ),
                        ],
                        ..Default::default()
                    },
                ),
                TheContextMenuItem::new("Noise".to_string(), TheId::named("Noise3D")),
                TheContextMenuItem::new("Material".to_string(), TheId::named("Material")),
            ],
            ..Default::default()
        }));

        let mut blend = TheSlider::new(TheId::named("ModelFX Blend"));
        blend.set_value(TheValue::Float(0.5));
        blend.set_default_value(TheValue::Float(0.5));
        blend.set_range(TheValue::RangeF32(0.0..=1.0));
        blend.set_continuous(true);
        blend.limiter_mut().set_max_width(120);
        blend.set_status_text("Sets the blend factor for the preview in the 2D Map. 0 only shows the conceptual preview, 1 the fully rendered preview.");

        // toolbar_hlayout.add_widget(Box::new(clear_button));
        // toolbar_hlayout.add_widget(Box::new(move_button));
        toolbar_hlayout.add_widget(Box::new(gb));

        let mut spacer = TheSpacer::new(TheId::empty());
        spacer.limiter_mut().set_max_size(vec2i(40, 5));
        toolbar_hlayout.add_widget(Box::new(spacer));

        // toolbar_hlayout.add_widget(Box::new(floors_button));
        // toolbar_hlayout.add_widget(Box::new(walls_button));
        toolbar_hlayout.add_widget(Box::new(material_button));
        toolbar_hlayout.add_widget(Box::new(blend));
        toolbar_hlayout.set_reverse_index(Some(1));

        /*
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
        */

        toolbar_canvas.set_layout(toolbar_hlayout);

        canvas.set_top(toolbar_canvas);

        // ModelFX Settings (Right)

        let mut settings_canvas = TheCanvas::default();

        let mut text_layout = TheTextLayout::new(TheId::named("ModelFX Settings"));
        //text_layout.set_fixed_text_width(90);
        text_layout.limiter_mut().set_max_width(240);
        settings_canvas.set_layout(text_layout);

        canvas.set_right(settings_canvas);

        // - ModelFX View

        let mut modelfx_stack = TheStackLayout::new(TheId::named("ModelFX Stack"));

        let mut geometry_canvas = TheCanvas::new();
        let mut rgba_layout = TheRGBALayout::new(TheId::named("GeoFX RGBA Layout"));
        if let Some(rgba_view) = rgba_layout.rgba_view_mut().as_rgba_view() {
            rgba_view.set_grid(Some(24));
            rgba_view.set_mode(TheRGBAViewMode::TilePicker);
            let mut c = WHITE;
            c[3] = 128;
            rgba_view.set_hover_color(Some(c));
        }
        geometry_canvas.set_layout(rgba_layout);

        let mut material_canvas = TheCanvas::new();
        let mut rgba_layout = TheRGBALayout::new(TheId::named("MaterialFX RGBA Layout"));
        if let Some(rgba_view) = rgba_layout.rgba_view_mut().as_rgba_view() {
            rgba_view.set_grid(Some(48));
            rgba_view.set_mode(TheRGBAViewMode::TilePicker);
            let mut c = WHITE;
            c[3] = 128;
            rgba_view.set_hover_color(Some(c));
        }
        material_canvas.set_layout(rgba_layout);

        let mut texture_node_canvas = TheCanvas::new();
        let node_view = TheNodeCanvasView::new(TheId::named("MaterialFX NodeCanvas"));
        texture_node_canvas.set_widget(node_view);

        modelfx_stack.add_canvas(geometry_canvas);
        modelfx_stack.add_canvas(material_canvas);
        modelfx_stack.add_canvas(texture_node_canvas);

        canvas.set_layout(modelfx_stack);

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
            TheEvent::SizeChanged(id) => {
                if id.name == "GeoFX RGBA Layout" {
                    self.set_geo_tiles(ui, ctx);
                } else if id.name == "MaterialFX RGBA Layout" {
                    self.set_material_tiles(ui, ctx, project, None);
                }
            }
            TheEvent::StateChanged(id, state) => {
                if id.name == "MaterialFX Add" {
                    let mut material = MaterialFXObject::default();
                    let node = MaterialFXNode::new(MaterialFXNodeRole::Material);
                    material.nodes.push(node);
                    material.selected_node = Some(material.nodes.len() - 1);

                    PRERENDERTHREAD
                        .lock()
                        .unwrap()
                        .material_changed(material.clone());
                    // if let Some(region) = project.get_region(&server_ctx.curr_region) {
                    //     let area = region.get_material_area(material.id);
                    //     PRERENDERTHREAD.lock().unwrap().render_region(
                    //         region.clone(),
                    //         project.palette.clone(),
                    //         area,
                    //     );
                    // }
                    let material_id = material.id;
                    project.materials.insert(material.id, material);
                    server_ctx.curr_material_object = Some(material_id);
                    self.set_material_tiles(ui, ctx, project, Some(material_id));
                    self.set_material_node_ui(server_ctx, project, ui, ctx);
                } else if id.name == "ModelFX Clear" && state == &TheWidgetState::Clicked {
                    self.modelfx = ModelFX::default();
                    self.modelfx.draw(ui, ctx, &project.palette);
                    self.render_preview(ui, &project.palette);
                    redraw = true;
                } else if id.name == "ModelFX Move" && state == &TheWidgetState::Clicked {
                    if !self.modelfx.nodes.is_empty() {
                        project.models.push(self.modelfx.clone());
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("ModelFX Redraw Library"),
                            TheValue::Empty,
                        ));
                        self.redraw_modelfx_library(project, ui, ctx);
                    }
                    redraw = true;
                }
            }
            TheEvent::ContextMenuSelected(_id, item) => {
                //let prev = self.modelfx.to_json();
                if self.editing_mode == EditingMode::MaterialNodes
                    && self.modelfx.add(item.name.clone())
                {
                    if let Some(material_id) = server_ctx.curr_material_object {
                        if let Some(material) = project.materials.get_mut(&material_id) {
                            let material_id = material.id;
                            let node = MaterialFXNode::new(MaterialFXNodeRole::Material);
                            material.nodes.push(node);
                            material.selected_node = Some(material.nodes.len() - 1);
                            let node_canvas = material.to_canvas(&project.palette);
                            ui.set_node_canvas("MaterialFX NodeCanvas", node_canvas);
                            self.set_material_tiles(ui, ctx, project, Some(material_id));
                        }
                    }
                    /*
                    self.modelfx.draw(ui, ctx, &project.palette);
                    self.update_node_canvas(&project.palette, ui);
                    self.set_selected_node_ui(server_ctx, project, ui, ctx);
                    self.render_preview(ui, &project.palette);
                    let undo = ModelFXUndoAtom::AddNode(prev, self.modelfx.to_json());
                    UNDOMANAGER.lock().unwrap().add_modelfx_undo(undo, ctx);*/
                    redraw = true;
                }
            }
            TheEvent::TilePicked(id, coord) => {
                if id.name == "GeoFX RGBA Layout View" {
                    server_ctx.curr_geo_object = None;
                    server_ctx.curr_geo_node = None;
                    self.set_geo_node_ui(server_ctx, project, ui, ctx);
                } else if id.name == "MaterialFX RGBA Layout View" {
                    if let Some(material) = self.materials.get(&(coord.x, coord.y)) {
                        server_ctx.curr_material_object = Some(*material);

                        let mut region_to_render: Option<Region> = None;
                        let mut tiles_to_render: Vec<Vec2i> = vec![];

                        if let Some(curr_geo_node) = server_ctx.curr_geo_node {
                            if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                                if let Some((geo_obj, _)) = region.find_geo_node(curr_geo_node) {
                                    geo_obj.material_id = *material;
                                    geo_obj.update_area();

                                    tiles_to_render.clone_from(&geo_obj.area);

                                    server.update_region(region);
                                    region_to_render = Some(region.clone());
                                }
                            }
                        }

                        // Render the region area covered by the object with the new material.
                        if let Some(region) = region_to_render {
                            PRERENDERTHREAD.lock().unwrap().render_region(
                                region,
                                project.palette.clone(),
                                tiles_to_render,
                            );
                        }
                    } else {
                        server_ctx.curr_material_object = None;
                    }
                    self.set_material_node_ui(server_ctx, project, ui, ctx);
                    redraw = true;
                }
            }
            TheEvent::TileEditorClicked(id, coord) => {
                if id.name == "GeoFX RGBA Layout View" && self.modelfx.clicked(*coord, ui, ctx) {
                    //self.modelfx.draw(ui, ctx, &project.palette);
                    self.set_selected_node_ui(server_ctx, project, ui, ctx);
                    self.render_preview(ui, &project.palette);
                    redraw = true;
                }
            }
            TheEvent::TileEditorDragged(id, coord) => {
                if id.name == "GeoFX RGBA Layout View" && self.modelfx.dragged(*coord, ui, ctx) {
                    //self.modelfx.draw(ui, ctx, &project.palette);
                    redraw = true;
                }
            }
            TheEvent::TileEditorUp(id) => {
                //let prev = self.modelfx.to_json();
                if id.name == "GeoFX RGBA Layout View" && self.modelfx.released(ui, ctx) {
                    //self.modelfx.draw(ui, ctx, &project.palette);
                    self.render_preview(ui, &project.palette);
                    //let undo = ModelFXUndoAtom::Edit(prev, self.modelfx.to_json());
                    //UNDOMANAGER.lock().unwrap().add_modelfx_undo(undo, ctx);
                    redraw = true;
                }
            }
            TheEvent::TileEditorHoverChanged(id, coord) => {
                if id.name == "GeoFX RGBA Layout View" && self.modelfx.hovered(*coord, ui, ctx) {
                    redraw = true;
                }
            }
            TheEvent::TileEditorDelete(id, selected) => {
                if id.name == "ModelFX Library RGBA Layout View" {
                    let prev = self.modelfx.to_json();
                    for pos in selected {
                        let index = (pos.0 + pos.1 * 4) as usize;
                        project.models.remove(index);
                    }
                    self.redraw_modelfx_library(project, ui, ctx);
                    let undo = ModelFXUndoAtom::Edit(prev, self.modelfx.to_json());
                    UNDOMANAGER.lock().unwrap().add_modelfx_undo(undo, ctx);
                    redraw = true;
                } else if id.name == "GeoFX RGBA Layout View" {
                    let prev = self.modelfx.to_json();
                    self.modelfx.delete();
                    self.modelfx.draw(ui, ctx, &project.palette);
                    self.render_preview(ui, &project.palette);
                    let undo = ModelFXUndoAtom::Edit(prev, self.modelfx.to_json());
                    UNDOMANAGER.lock().unwrap().add_modelfx_undo(undo, ctx);
                    redraw = true;
                }
            }
            TheEvent::ColorButtonClicked(id) => {
                // When a color button is clicked, copy over the current palette index.
                if id.name.starts_with(":MODELFX:") {
                    //let prev = self.modelfx.to_json();
                    if let Some(name) = id.name.strip_prefix(":MODELFX: ") {
                        if let Some(material_id) = server_ctx.curr_material_object {
                            if let Some(material) = project.materials.get_mut(&material_id) {
                                if let Some(selected_index) = material.selected_node {
                                    if let Some(color) = project.palette.get_current_color() {
                                        let mut old_index = None;
                                        if let Some(TheValue::PaletteIndex(index)) =
                                            material.nodes[selected_index].get(name)
                                        {
                                            old_index = Some(index);
                                        }

                                        material.nodes[selected_index].set(
                                            name,
                                            TheValue::PaletteIndex(project.palette.current_index),
                                        );

                                        PRERENDERTHREAD
                                            .lock()
                                            .unwrap()
                                            .material_changed(material.clone());

                                        if let Some(region) =
                                            project.get_region(&server_ctx.curr_region)
                                        {
                                            let area = region.get_material_area(material_id);
                                            PRERENDERTHREAD.lock().unwrap().render_region(
                                                region.clone(),
                                                project.palette.clone(),
                                                area,
                                            );
                                        }

                                        if let Some(widget) = ui.get_widget(&id.name) {
                                            widget.set_value(TheValue::ColorObject(color));
                                        }

                                        if let Some(old_index) = old_index {
                                            // Insert the new relationship
                                            let new_index = project.palette.current_index;
                                            if let Some(indices) =
                                                self.palette_indices.get_mut(&id.name)
                                            {
                                                for index in indices.iter_mut() {
                                                    if *index == old_index {
                                                        *index = new_index;
                                                        break;
                                                    }
                                                }
                                            }
                                        }
                                        // self.modelfx.clear_previews();
                                        // self.modelfx.draw(ui, ctx, &project.palette);
                                        // self.render_preview(ui, &project.palette);
                                        // let undo =
                                        //     ModelFXUndoAtom::Edit(prev, self.modelfx.to_json());
                                        // UNDOMANAGER.lock().unwrap().add_modelfx_undo(undo, ctx);
                                        // redraw = true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            /*
            TheEvent::TimelineMarkerSelected(id, time) => {
                if id.name == "ModelFX Timeline" {
                    self.curr_marker = Some(*time);
                    redraw = true;
                }
                }*/
            TheEvent::ValueChanged(id, value) => {
                if id.name == "ModelFX Blend" {
                    if let TheValue::Float(value) = value {
                        server_ctx.conceptual_display = Some(*value);
                    }
                } else if id.name == "Palette Color Picker" {
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
                    self.modelfx.clear_previews();
                    self.modelfx.draw(ui, ctx, &project.palette);
                    self.render_preview(ui, &project.palette);
                } else if id.name.starts_with(":MODELFX:") {
                    if let Some(name) = id.name.strip_prefix(":MODELFX: ") {
                        let mut value = value.clone();

                        if let TheValue::Text(_) = &value {
                            if let Some(v) = value.to_f32() {
                                value = TheValue::Float(v);
                            }
                        }

                        if self.editing_mode == EditingMode::Geometry {
                            if let Some(curr_geo_node) = server_ctx.curr_geo_node {
                                let mut region_to_render: Option<Region> = None;
                                let mut old_tiles_to_render: Vec<Vec2i> = vec![];
                                let mut new_tiles_to_render: Vec<Vec2i> = vec![];
                                let mut tiles_to_render: Vec<Vec2i> = vec![];

                                if let Some(region) =
                                    project.get_region_mut(&server_ctx.curr_region)
                                {
                                    if let Some((geo_obj, index)) =
                                        region.find_geo_node(curr_geo_node)
                                    {
                                        old_tiles_to_render.clone_from(&geo_obj.area);
                                        geo_obj.nodes[index].set(name, value);
                                        geo_obj.update_area();

                                        new_tiles_to_render.clone_from(&geo_obj.area);
                                        let mut set: FxHashSet<Vec2i> = FxHashSet::default();
                                        set.extend(&old_tiles_to_render);
                                        set.extend(&new_tiles_to_render);
                                        tiles_to_render = set.into_iter().collect();

                                        region.update_geometry_areas();

                                        region_to_render = Some(region.clone());

                                        server.update_region(region);
                                    }
                                }

                                if let Some(region) = region_to_render {
                                    PRERENDERTHREAD.lock().unwrap().render_region(
                                        region,
                                        project.palette.clone(),
                                        tiles_to_render,
                                    );
                                }
                            } else if let Some(editor) = ui.get_rgba_layout("GeoFX RGBA Layout") {
                                if let Some(rgba_view) = editor.rgba_view_mut().as_rgba_view() {
                                    let selection = rgba_view.selection();
                                    for i in selection {
                                        if let Some(geo) = self.geos.get_mut(&i) {
                                            geo.set(name, value);
                                            break;
                                        }
                                    }
                                }
                            }
                        } else if self.editing_mode == EditingMode::MaterialNodes {
                            println!("mat {:?}", value);
                        }
                        /*
                        let prev = self.modelfx.to_json();
                        if let Some(selected_index) = self.modelfx.selected_node {
                            let collection = self.modelfx.nodes[selected_index].collection_mut();

                            // Correct values to their range variants if necessary as TheSlider strips them
                            // of the range
                            if let Some(TheValue::FloatRange(_, range)) = collection.get(name) {
                                if let Some(v) = value.to_f32() {
                                    value = TheValue::FloatRange(v, range.clone());
                                }
                            } else if let Some(TheValue::IntRange(_, range)) = collection.get(name)
                            {
                                if let Some(v) = value.to_i32() {
                                    value = TheValue::IntRange(v, range.clone());
                                }
                            } else if let Some(TheValue::TextList(_, list)) = collection.get(name) {
                                if let Some(v) = value.to_i32() {
                                    value = TheValue::TextList(v, list.clone());
                                }
                            }

                            collection.set(name, value);

                            self.modelfx.remove_current_node_preview();
                            self.modelfx.draw(ui, ctx, &project.palette);
                            self.render_preview(ui, &project.palette);
                            let undo = ModelFXUndoAtom::Edit(prev, self.modelfx.to_json());
                            UNDOMANAGER.lock().unwrap().add_modelfx_undo(undo, ctx);
                            redraw = true;
                        }
                        */
                    }
                }
            }
            /*
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
                }*/
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
                }
            }
            TheEvent::IndexChanged(id, index) => {
                if id.name == "ModelFX Mode Group" {
                    if let Some(stack) = ui.get_stack_layout("ModelFX Stack") {
                        stack.set_index(*index);
                        redraw = true;
                        ctx.ui.relayout = true;
                        if *index == 0 {
                            self.editing_mode = EditingMode::Geometry;
                            self.set_geo_node_ui(server_ctx, project, ui, ctx);
                        } else if *index == 1 {
                            self.editing_mode = EditingMode::Material;
                            self.set_material_node_ui(server_ctx, project, ui, ctx);
                        } else {
                            self.editing_mode = EditingMode::MaterialNodes;
                            self.set_selected_node_ui(server_ctx, project, ui, ctx);
                        }
                    }
                }
            }
            TheEvent::NodeSelectedIndexChanged(id, index) => {
                if id.name == "MaterialFX NodeCanvas" {
                    if let Some(material_id) = server_ctx.curr_material_object {
                        if let Some(material) = project.materials.get_mut(&material_id) {
                            material.selected_node = *index;
                        }
                    }
                    self.set_selected_node_ui(server_ctx, project, ui, ctx);
                }
            }
            TheEvent::NodeDragged(id, index, position) => {
                if id.name == "MaterialFX NodeCanvas" {
                    if let Some(material_id) = server_ctx.curr_material_object {
                        if let Some(material) = project.materials.get_mut(&material_id) {
                            material.nodes[*index].position = *position;
                        }
                    }
                }
            }
            TheEvent::NodeConnectionAdded(id, connections)
            | TheEvent::NodeConnectionRemoved(id, connections) => {
                if id.name == "MaterialFX NodeCanvas" {
                    if let Some(material_id) = server_ctx.curr_material_object {
                        if let Some(material) = project.materials.get_mut(&material_id) {
                            material.connections.clone_from(connections);
                        }
                    }
                }
            }
            TheEvent::NodeDeleted(id, deleted_node_index, connections) => {
                if id.name == "MaterialFX NodeCanvas" {
                    if let Some(material_id) = server_ctx.curr_material_object {
                        if let Some(material) = project.materials.get_mut(&material_id) {
                            material.nodes.remove(*deleted_node_index);
                            material.node_previews.remove(*deleted_node_index);
                            material.connections.clone_from(connections);
                            redraw = true;
                        }
                    }
                }
            }

            _ => {}
        }

        redraw
    }

    /// Modeler got activated, set the UI
    pub fn activated(
        &mut self,
        server_ctx: &mut ServerContext,
        project: &mut Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) {
        if self.editing_mode == EditingMode::Geometry {
            self.set_geo_node_ui(server_ctx, project, ui, ctx);
        } else if self.editing_mode == EditingMode::Material {
            self.set_material_node_ui(server_ctx, project, ui, ctx);
        } else {
            self.set_selected_node_ui(server_ctx, project, ui, ctx);
        }
    }

    pub fn set_geo_node_ui(
        &mut self,
        server_ctx: &mut ServerContext,
        project: &mut Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) {
        self.palette_indices.clear();

        let mut collection: Option<TheCollection> = None;

        if let Some(curr_geo_node) = server_ctx.curr_geo_node {
            if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                if let Some((geo_obj, index)) = region.find_geo_node(curr_geo_node) {
                    collection = Some(geo_obj.nodes[index].collection());
                }
            }
        } else if let Some(geo) = self.get_geo_node(ui) {
            collection = Some(geo.collection());
        }

        if let Some(collection) = collection {
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
                        //slider.set_continuous(true);
                        text_layout.add_pair(name.clone(), Box::new(slider));
                    } else if let TheValue::Float(value) = value {
                        let mut slider = TheTextLineEdit::new(TheId::named(
                            (":MODELFX: ".to_owned() + name).as_str(),
                        ));
                        slider.set_value(TheValue::Float(*value));
                        text_layout.add_pair(name.clone(), Box::new(slider));
                    } else if let TheValue::IntRange(value, range) = value {
                        let mut slider = TheTextLineEdit::new(TheId::named(
                            (":MODELFX: ".to_owned() + name).as_str(),
                        ));
                        slider.set_value(TheValue::Int(*value));
                        slider.set_range(TheValue::RangeI32(range.clone()));
                        //slider.set_continuous(true);
                        text_layout.add_pair(name.clone(), Box::new(slider));
                    } else if let TheValue::TextList(index, list) = value {
                        let mut dropdown = TheDropdownMenu::new(TheId::named(
                            (":MODELFX: ".to_owned() + name).as_str(),
                        ));
                        for item in list {
                            dropdown.add_option(item.clone());
                        }
                        dropdown.set_selected_index(*index);
                        text_layout.add_pair(name.clone(), Box::new(dropdown));
                    } else if let TheValue::PaletteIndex(index) = value {
                        let name_id = ":MODELFX: ".to_owned() + name;
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
                    }
                }
                ctx.ui.relayout = true;
            }
        } else if let Some(text_layout) = ui.get_text_layout("ModelFX Settings") {
            text_layout.clear();
        }
    }

    pub fn set_material_node_ui(
        &mut self,
        server_ctx: &mut ServerContext,
        project: &mut Project,
        ui: &mut TheUI,
        _ctx: &mut TheContext,
    ) {
        if let Some(text_layout) = ui.get_text_layout("ModelFX Settings") {
            text_layout.clear();

            let mut add_button = TheTraybarButton::new(TheId::named("MaterialFX Add"));
            add_button.set_text("Add Material".to_string());
            add_button
                .set_status_text("Switches between an anim based preview and multi tiles preview.");

            text_layout.add_pair("".to_string(), Box::new(add_button));

            if let Some(materialid) = server_ctx.curr_material_object {
                if let Some(material) = project.materials.get_mut(&materialid) {
                    let node_canvas = material.to_canvas(&project.palette);
                    ui.set_node_canvas("MaterialFX NodeCanvas", node_canvas);
                }
            }
        }
    }

    pub fn set_selected_node_ui(
        &mut self,
        server_ctx: &mut ServerContext,
        project: &mut Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) {
        self.palette_indices.clear();

        if let Some(material_id) = server_ctx.curr_material_object {
            if let Some(material) = project.materials.get_mut(&material_id) {
                if let Some(selected_index) = material.selected_node {
                    let collection = material.nodes[selected_index].collection();

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
                                text_layout.add_pair(name.clone(), Box::new(slider));
                            } else if let TheValue::IntRange(value, range) = value {
                                let mut slider = TheTextLineEdit::new(TheId::named(
                                    (":MODELFX: ".to_owned() + name).as_str(),
                                ));
                                slider.set_value(TheValue::Int(*value));
                                slider.set_range(TheValue::RangeI32(range.clone()));
                                slider.set_continuous(true);
                                text_layout.add_pair(name.clone(), Box::new(slider));
                            } else if let TheValue::TextList(index, list) = value {
                                let mut dropdown = TheDropdownMenu::new(TheId::named(
                                    (":MODELFX: ".to_owned() + name).as_str(),
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
                PRERENDERTHREAD
                    .lock()
                    .unwrap()
                    .material_changed(material.clone());
                if let Some(region) = project.get_region(&server_ctx.curr_region) {
                    let area = region.get_material_area(material_id);
                    PRERENDERTHREAD.lock().unwrap().render_region(
                        region.clone(),
                        project.palette.clone(),
                        area,
                    );
                }
            }
        } else if let Some(text_layout) = ui.get_text_layout("ModelFX Settings") {
            text_layout.clear();
        }
    }

    pub fn render_preview(&mut self, ui: &mut TheUI, palette: &ThePalette) {
        self.modelfx.create_voxels(24, &Vec3f::zero(), palette);

        if *SIDEBARMODE.lock().unwrap() == SidebarMode::Model {
            if let Some(widget) = ui.get_render_view("ModelFX Library Preview") {
                let buffer = widget.render_buffer_mut();

                self.modelfx.render_preview(buffer, palette);
            }
        }

        if let Some(icon_view) = ui.get_icon_view("Icon Preview") {
            let mut buffer = TheRGBABuffer::new(TheDim::sized(65, 65));
            self.modelfx.render_preview(&mut buffer, palette);
            self.modelfx.preview_buffer = buffer.clone();
            let tile = TheRGBATile::buffer(buffer);
            icon_view.set_rgba_tile(tile);
        }
    }

    pub fn get_model(&self) -> ModelFX {
        self.modelfx.clone()
    }

    /*
    pub fn set_model(
        &mut self,
        model: ModelFX,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        palette: &ThePalette,
    ) {
        self.set_selected_node_ui(ui, ctx, palette);
        self.modelfx = model;
        self.update_node_canvas(palette, ui);
        self.modelfx.draw(ui, ctx, palette);
        UNDOMANAGER.lock().unwrap().clear_modelfx();
        self.render_preview(ui, palette);
    }
    */

    /// Update the node canvas
    fn _update_node_canvas(&mut self, palette: &ThePalette, ui: &mut TheUI) {
        ui.set_node_canvas("ModelFX NodeCanvas", self.modelfx.to_canvas(palette));
    }

    /// Set the library models.
    pub fn redraw_modelfx_library(
        &mut self,
        project: &Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) {
        if let Some(editor) = ui.get_rgba_layout("ModelFX Library RGBA Layout") {
            //println!("{}", editor.dim().width);
            let width = 275; //editor.dim().width - 16;
            let height = editor.dim().height - 16;
            if let Some(rgba_view) = editor.rgba_view_mut().as_rgba_view() {
                let grid = 65;

                rgba_view.set_grid(Some(grid));

                let tiles_per_row = width / grid;
                let lines = project.models.len() as i32 / tiles_per_row + 1;

                let mut buffer =
                    TheRGBABuffer::new(TheDim::sized(width, max(lines * grid, height)));
                buffer.fill([74, 74, 74, 255]);

                for (i, model) in project.models.iter().enumerate() {
                    let x = i as i32 % tiles_per_row;
                    let y = i as i32 / tiles_per_row;

                    buffer.copy_into(x * grid, y * grid, &model.preview_buffer);
                }
                rgba_view.set_buffer(buffer);
            }
            editor.relayout(ctx);
        }
    }

    /// Set the tiles for the picker.
    pub fn set_geo_tiles(&mut self, ui: &mut TheUI, _ctx: &mut TheContext) {
        let tile_size = 48;

        //let mut set_default_selection = false;

        let geo_tiles = if self.geos.is_empty() {
            //set_default_selection = true;
            GeoFXNode::nodes()
        } else {
            self.geos.values().cloned().collect()
        };

        self.geos.clear();

        if let Some(editor) = ui.get_rgba_layout("GeoFX RGBA Layout") {
            if editor.dim().width == 0 {
                return;
            }

            let width = editor.dim().width - 16;
            let height = editor.dim().height - 16;

            if let Some(rgba_view) = editor.rgba_view_mut().as_rgba_view() {
                let grid = tile_size;

                rgba_view.set_grid(Some(grid));

                let tiles_per_row = width / grid;
                let lines = geo_tiles.len() as i32 / tiles_per_row + 1;

                let mut buffer =
                    TheRGBABuffer::new(TheDim::sized(width, max(lines * grid, height)));

                let mut tile_buffer = TheRGBABuffer::new(TheDim::sized(tile_size, tile_size));

                for (i, tile) in geo_tiles.iter().enumerate() {
                    let x = i as i32 % tiles_per_row;
                    let y = i as i32 / tiles_per_row;

                    /*
                    self.tile_ids.insert((x, y), tile.id);
                    self.tile_text.insert(
                        (x, y),
                        format!(
                            "{} : {}",
                            tile.name,
                            TileRole::from_index(tile.role)
                                .unwrap_or(TileRole::ManMade)
                                .to_string()
                        ),
                    );
                    if !tile.buffer.is_empty() {
                        buffer.copy_into(x * grid, y * grid, &tile.buffer[0].scaled(grid, grid));
                        }*/

                    tile.preview(&mut tile_buffer);
                    buffer.copy_into(x * grid, y * grid, &tile_buffer);
                    self.geos.insert((x, y), tile.clone());
                }

                rgba_view.set_buffer(buffer);
                // if set_default_selection {
                //     let mut hashset = FxHashSet::default();
                //     hashset.insert((0, 0));
                //     rgba_view.set_selection(hashset);
                // }
            }
        }
    }

    /// Get the currently selected geometry node.
    pub fn get_geo_node(&self, ui: &mut TheUI) -> Option<GeoFXNode> {
        if let Some(editor) = ui.get_rgba_layout("GeoFX RGBA Layout") {
            if let Some(rgba_view) = editor.rgba_view_mut().as_rgba_view() {
                let selection = rgba_view.selection();

                for i in selection {
                    if let Some(tile) = self.geos.get(&i) {
                        return Some(tile.clone());
                    }
                }
            }
        }

        None
    }

    /// Set the tiles for the picker.
    pub fn set_material_tiles(
        &mut self,
        ui: &mut TheUI,
        _ctx: &mut TheContext,
        project: &Project,
        set_selection: Option<Uuid>,
    ) {
        let tile_size = 48;

        self.materials.clear();

        if let Some(editor) = ui.get_rgba_layout("MaterialFX RGBA Layout") {
            if editor.dim().width == 0 {
                return;
            }

            let width = editor.dim().width - 16;
            let height = editor.dim().height - 16;

            if let Some(rgba_view) = editor.rgba_view_mut().as_rgba_view() {
                let grid = tile_size;

                rgba_view.set_grid(Some(grid));

                let tiles_per_row = width / grid;
                let lines = project.materials.len() as i32 / tiles_per_row + 1;

                let mut buffer =
                    TheRGBABuffer::new(TheDim::sized(width, max(lines * grid, height)));

                let tile_buffer = TheRGBABuffer::new(TheDim::sized(tile_size, tile_size));

                for (i, (id, obj)) in project.materials.iter().enumerate() {
                    let x = i as i32 % tiles_per_row;
                    let y = i as i32 / tiles_per_row;

                    if Some(obj.id) == set_selection {
                        let mut hashset = FxHashSet::default();
                        hashset.insert((x, y));
                        rgba_view.set_selection(hashset);
                    }
                    /*
                    self.tile_ids.insert((x, y), tile.id);
                    self.tile_text.insert(
                        (x, y),
                        format!(
                            "{} : {}",
                            tile.name,
                            TileRole::from_index(tile.role)
                                .unwrap_or(TileRole::ManMade)
                                .to_string()
                        ),
                    );
                    if !tile.buffer.is_empty() {
                        buffer.copy_into(x * grid, y * grid, &tile.buffer[0].scaled(grid, grid));
                        }*/

                    //tile.preview(&mut tile_buffer);
                    buffer.copy_into(x * grid, y * grid, &tile_buffer);
                    self.materials.insert((x, y), *id);
                }

                rgba_view.set_buffer(buffer);
            }
        }
    }
}
