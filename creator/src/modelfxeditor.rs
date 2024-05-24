use crate::editor::{SIDEBARMODE, UNDOMANAGER};
use crate::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ModelFXMode {
    Floor,
    Wall,
    Ceiling,
}

pub struct ModelFXEditor {
    pub mode: ModelFXMode,
    pub geo_names: FxHashMap<(i32, i32), String>,

    pub modelfx: ModelFX,

    pub palette_indices: FxHashMap<String, Vec<u16>>,
}

#[allow(clippy::new_without_default)]
impl ModelFXEditor {
    pub fn new() -> Self {
        Self {
            mode: ModelFXMode::Floor,
            geo_names: FxHashMap::default(),

            modelfx: ModelFX::default(),

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
        gb.add_text("Geometry".to_string());
        gb.add_text("Texture".to_string());
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

        let mut zoom = TheSlider::new(TheId::named("ModelFX Zoom"));
        zoom.set_value(TheValue::Float(1.0));
        zoom.set_default_value(TheValue::Float(1.0));
        zoom.set_range(TheValue::RangeF32(1.0..=5.0));
        zoom.set_continuous(true);
        zoom.limiter_mut().set_max_width(120);

        // toolbar_hlayout.add_widget(Box::new(clear_button));
        // toolbar_hlayout.add_widget(Box::new(move_button));
        toolbar_hlayout.add_widget(Box::new(gb));

        let mut spacer = TheSpacer::new(TheId::empty());
        spacer.limiter_mut().set_max_size(vec2i(40, 5));
        toolbar_hlayout.add_widget(Box::new(spacer));

        toolbar_hlayout.add_widget(Box::new(floors_button));
        toolbar_hlayout.add_widget(Box::new(walls_button));
        toolbar_hlayout.add_widget(Box::new(material_button));
        toolbar_hlayout.add_widget(Box::new(zoom));
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
        let mut rgba_layout = TheRGBALayout::new(TheId::named("ModelFX RGBA Layout"));
        if let Some(rgba_view) = rgba_layout.rgba_view_mut().as_rgba_view() {
            rgba_view.set_grid(Some(24));
            rgba_view.set_mode(TheRGBAViewMode::TilePicker);
            let mut c = WHITE;
            c[3] = 128;
            rgba_view.set_hover_color(Some(c));
        }
        geometry_canvas.set_layout(rgba_layout);

        let mut texture_node_canvas = TheCanvas::new();
        let node_view = TheNodeCanvasView::new(TheId::named("ModelFX NodeCanvas"));
        texture_node_canvas.set_widget(node_view);

        modelfx_stack.add_canvas(geometry_canvas);
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
        _server: &mut Server,
        _server_ctx: &mut ServerContext,
    ) -> bool {
        let mut redraw = false;

        match event {
            TheEvent::SizeChanged(id) => {
                if id.name == "ModelFX RGBA Layout" {
                    self.set_geo_tiles(ui, ctx);
                }
            }
            TheEvent::StateChanged(id, state) => {
                if id.name == "ModelFX Clear" && state == &TheWidgetState::Clicked {
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
                let prev = self.modelfx.to_json();
                if
                /*id.name.starts_with("ModelFX Node") &&*/
                self.modelfx.add(item.name.clone()) {
                    self.modelfx.draw(ui, ctx, &project.palette);
                    self.update_node_canvas(&project.palette, ui);
                    self.set_selected_node_ui(ui, ctx, &project.palette);
                    self.render_preview(ui, &project.palette);
                    let undo = ModelFXUndoAtom::AddNode(prev, self.modelfx.to_json());
                    UNDOMANAGER.lock().unwrap().add_modelfx_undo(undo, ctx);
                    redraw = true;
                }
            }
            TheEvent::TilePicked(id, coord) => {
                if id.name == "ModelFX Library RGBA Layout View" {
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Set Region Modeler"),
                        TheValue::Empty,
                    ));

                    if let Some(button) = ui.get_group_button("Editor Group") {
                        button.set_index(EditorMode::Model as i32);
                        ctx.ui.send(TheEvent::IndexChanged(
                            button.id().clone(),
                            EditorMode::Model as usize,
                        ));
                    }

                    let index = coord.x + coord.y * 4;
                    if let Some(model) = project.models.get(index as usize) {
                        self.modelfx = model.clone();
                        self.modelfx.draw(ui, ctx, &project.palette);
                        self.set_selected_node_ui(ui, ctx, &project.palette);
                        self.render_preview(ui, &project.palette);
                        redraw = true;
                    }
                }
            }
            TheEvent::TileEditorClicked(id, coord) => {
                if id.name == "ModelFX RGBA Layout View" && self.modelfx.clicked(*coord, ui, ctx) {
                    self.modelfx.draw(ui, ctx, &project.palette);
                    self.set_selected_node_ui(ui, ctx, &project.palette);
                    self.render_preview(ui, &project.palette);
                    redraw = true;
                }
            }
            TheEvent::TileEditorDragged(id, coord) => {
                if id.name == "ModelFX RGBA Layout View" && self.modelfx.dragged(*coord, ui, ctx) {
                    self.modelfx.draw(ui, ctx, &project.palette);
                    redraw = true;
                }
            }
            TheEvent::TileEditorUp(id) => {
                let prev = self.modelfx.to_json();
                if id.name == "ModelFX RGBA Layout View" && self.modelfx.released(ui, ctx) {
                    self.modelfx.draw(ui, ctx, &project.palette);
                    self.render_preview(ui, &project.palette);
                    let undo = ModelFXUndoAtom::Edit(prev, self.modelfx.to_json());
                    UNDOMANAGER.lock().unwrap().add_modelfx_undo(undo, ctx);
                    redraw = true;
                }
            }
            TheEvent::TileEditorHoverChanged(id, coord) => {
                if id.name == "ModelFX RGBA Layout View" && self.modelfx.hovered(*coord, ui, ctx) {
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
                } else if id.name == "ModelFX RGBA Layout View" {
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
                    let prev = self.modelfx.to_json();
                    if let Some(name) = id.name.strip_prefix(":MODELFX: ") {
                        if let Some(selected_index) = self.modelfx.selected_node {
                            let collection = self.modelfx.nodes[selected_index].collection_mut();

                            if let Some(color) = project.palette.get_current_color() {
                                let mut old_index = None;
                                if let Some(TheValue::PaletteIndex(index)) = collection.get(name) {
                                    old_index = Some(*index);
                                }

                                collection.set(
                                    name,
                                    TheValue::PaletteIndex(project.palette.current_index),
                                );

                                if let Some(widget) = ui.get_widget(&id.name) {
                                    widget.set_value(TheValue::ColorObject(color));
                                }

                                if let Some(old_index) = old_index {
                                    // Insert the new relationship
                                    let new_index = project.palette.current_index;
                                    if let Some(indices) = self.palette_indices.get_mut(&id.name) {
                                        for index in indices.iter_mut() {
                                            if *index == old_index {
                                                *index = new_index;
                                                break;
                                            }
                                        }
                                    }
                                }
                                self.modelfx.clear_previews();
                                self.modelfx.draw(ui, ctx, &project.palette);
                                self.render_preview(ui, &project.palette);
                                let undo = ModelFXUndoAtom::Edit(prev, self.modelfx.to_json());
                                UNDOMANAGER.lock().unwrap().add_modelfx_undo(undo, ctx);
                                redraw = true;
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
                if id.name == "ModelFX Zoom" {
                    if let TheValue::Float(value) = value {
                        self.modelfx.zoom = *value;
                        self.modelfx.draw(ui, ctx, &project.palette);
                        self.update_node_canvas(&project.palette, ui);
                        redraw = true;
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
                    }
                }
            }
            TheEvent::NodeSelectedIndexChanged(id, index) => {
                if id.name == "ModelFX NodeCanvas" {
                    self.modelfx.selected_node = *index;
                    self.set_selected_node_ui(ui, ctx, &project.palette);
                }
            }
            TheEvent::NodeDragged(id, index, position) => {
                if id.name == "ModelFX NodeCanvas" {
                    let collection = self.modelfx.nodes[*index].collection_mut();
                    collection.set("_pos", TheValue::Int2(*position));
                }
            }
            TheEvent::NodeConnectionAdded(id, connections)
            | TheEvent::NodeConnectionRemoved(id, connections) => {
                if id.name == "ModelFX NodeCanvas" {
                    self.modelfx.connections.clone_from(connections);
                }
            }
            TheEvent::NodeDeleted(id, deleted_node_index, connections) => {
                if id.name == "ModelFX NodeCanvas" {
                    self.modelfx.nodes.remove(*deleted_node_index);
                    self.modelfx.node_previews.remove(*deleted_node_index);
                    self.modelfx.connections.clone_from(connections);
                    redraw = true;
                }
            }

            _ => {}
        }

        redraw
    }

    pub fn set_selected_node_ui(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        palette: &ThePalette,
    ) {
        self.palette_indices.clear();

        if let Some(selected_index) = self.modelfx.selected_node {
            let collection = self.modelfx.nodes[selected_index].collection();

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
                        let mut color_picker = TheColorButton::new(TheId::named(name_id.as_str()));
                        color_picker.limiter_mut().set_max_size(vec2i(80, 20));
                        if let Some(color) = &palette[*index as usize] {
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

    /// Update the node canvas
    fn update_node_canvas(&mut self, palette: &ThePalette, ui: &mut TheUI) {
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
        self.geo_names.clear();
        let tile_size = 48;

        let geo_tiles = GeoFXNode::nodes();

        if let Some(editor) = ui.get_rgba_layout("ModelFX RGBA Layout") {
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
                }

                rgba_view.set_buffer(buffer);
            }
        }
    }
}
