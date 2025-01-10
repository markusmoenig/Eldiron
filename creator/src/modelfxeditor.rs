use crate::editor::{ACTIVEEDITOR, BRUSHLIST, TILEDRAWER, UNDOMANAGER};
use crate::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ModelFXMode {
    Floor,
    Wall,
    Ceiling,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MaterialMode {
    Material,
    MaterialNodes,
}

pub struct ModelFXEditor {
    pub mode: ModelFXMode,
    pub geos: FxHashMap<(i32, i32), GeoFXNode>,
    pub geos_description: FxHashMap<(i32, i32), String>,

    pub materials: FxHashMap<(i32, i32), Uuid>,

    pub geometry_mode: bool,
    pub material_mode: MaterialMode,

    pub curr_layer_role: Layer2DRole,

    pub current_material: Option<Uuid>,

    pub palette_indices: FxHashMap<String, Vec<u16>>,

    pub brush_size: f32,
    pub falloff: f32,
}

#[allow(clippy::new_without_default)]
impl ModelFXEditor {
    pub fn new() -> Self {
        Self {
            mode: ModelFXMode::Floor,
            geos: FxHashMap::default(),
            geos_description: FxHashMap::default(),
            materials: FxHashMap::default(),

            geometry_mode: true,
            material_mode: MaterialMode::Material,
            curr_layer_role: Layer2DRole::Wall,

            current_material: None,

            palette_indices: FxHashMap::default(),

            brush_size: 1.0,
            falloff: 0.5,
        }
    }

    /// Build the UI
    pub fn build_mapobjects(&self, _ctx: &mut TheContext) -> TheCanvas {
        let mut canvas = TheCanvas::new();

        // Toolbar
        let mut toolbar_canvas = TheCanvas::default();
        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.limiter_mut().set_max_height(25);
        toolbar_hlayout.set_margin(vec4i(10, 2, 5, 3));

        let mut blend = TheSlider::new(TheId::named("ModelFX Blend"));
        blend.set_value(TheValue::Float(0.5));
        blend.set_default_value(TheValue::Float(0.5));
        blend.set_range(TheValue::RangeF32(0.0..=1.0));
        blend.set_continuous(true);
        blend.limiter_mut().set_max_width(120);
        blend.set_status_text("Sets the blend factor for the preview in the 2D Map. 0 only shows the conceptual preview, 1 the fully rendered preview.");

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

        // - ModelFX View

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
        canvas.set_center(geometry_canvas);

        canvas
    }

    /// Build the UI
    pub fn build_brush_ui(
        &self,
        _project: &Project,
        _ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> TheCanvas {
        let mut canvas = TheCanvas::new();

        // Toolbar
        let mut toolbar_canvas = TheCanvas::default();
        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.limiter_mut().set_max_height(25);
        toolbar_hlayout.set_margin(vec4i(10, 2, 5, 3));

        let mut add_material_button: TheTraybarButton =
            TheTraybarButton::new(TheId::named("MaterialFX Add"));
        add_material_button.set_text("Add Material".to_string());
        add_material_button.set_status_text("Add a new material.");

        // Brush Size

        // let mut text = TheText::new(TheId::empty());
        // text.set_text("Brush Size".to_string());
        // toolbar_hlayout.add_widget(Box::new(text));

        let mut brush_size = TheTextLineEdit::new(TheId::named("Brush Size"));
        brush_size.set_value(TheValue::Float(self.brush_size));
        //brush_size.set_default_value(TheValue::Float(1.0));
        brush_size.set_info_text(Some("Brush Size".to_string()));
        brush_size.set_range(TheValue::RangeF32(0.01..=5.0));
        brush_size.set_continuous(true);
        brush_size.limiter_mut().set_max_width(170);
        brush_size.set_status_text("The brush size.");
        toolbar_hlayout.add_widget(Box::new(brush_size));

        // Falloff

        // let mut text = TheText::new(TheId::empty());
        // text.set_text("Falloff".to_string());
        // toolbar_hlayout.add_widget(Box::new(text));

        let mut falloff = TheTextLineEdit::new(TheId::named("Falloff"));
        falloff.set_value(TheValue::Float(self.falloff));
        //falloff.set_default_value(TheValue::Float(0.0));
        falloff.set_info_text(Some("Falloff".to_string()));
        falloff.set_range(TheValue::RangeF32(0.0..=1.0));
        falloff.set_continuous(true);
        falloff.limiter_mut().set_max_width(170);
        falloff.set_status_text("The falloff off the brush.");
        toolbar_hlayout.add_widget(Box::new(falloff));

        // let mut blend = TheSlider::new(TheId::named("ModelFX Blend"));
        // blend.set_value(TheValue::Float(0.5));
        // blend.set_default_value(TheValue::Float(0.5));
        // blend.set_range(TheValue::RangeF32(0.0..=1.0));
        // blend.set_continuous(true);
        // blend.limiter_mut().set_max_width(120);
        // blend.set_status_text("Sets the blend factor for the preview in the 2D Map. 0 only shows the conceptual preview, 1 the fully rendered preview.");

        // toolbar_hlayout.add_widget(Box::new(move_button));
        //toolbar_hlayout.add_widget(Box::new(gb));

        let mut spacer = TheSpacer::new(TheId::empty());
        spacer.limiter_mut().set_max_size(vec2i(40, 5));
        toolbar_hlayout.add_widget(Box::new(spacer));

        // toolbar_hlayout.add_widget(Box::new(floors_button));
        // toolbar_hlayout.add_widget(Box::new(walls_button));

        //toolbar_hlayout.add_widget(Box::new(add_material_button));

        //toolbar_hlayout.add_widget(Box::new(material_button));
        //toolbar_hlayout.add_widget(Box::new(blend));
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

        // - ModelFX View

        /*
        let mut modelfx_stack = TheStackLayout::new(TheId::named("ModelFX Stack"));

        // Materials
        let mut material_canvas = TheCanvas::new();
        let mut material_rowlist_layout =
            TheRowListLayout::new(TheId::named("ModelFX Material List"));

        for (index, material) in project.materials.values().enumerate() {
            let mut item = TheRowListItem::new(TheId::named_with_id(&material.name, material.id));
            item.set_text(material.name.clone());

            if index == 0 {
                item.set_state(TheWidgetState::Selected);
                server_ctx.curr_material_object = Some(material.id);
            }

            material_rowlist_layout.add_item(item, _ctx);
        }

        material_canvas.set_layout(material_rowlist_layout);

        // Material Editor
        let mut texture_node_canvas = TheCanvas::new();
        let node_view = TheNodeCanvasView::new(TheId::named("MaterialFX NodeCanvas"));
        texture_node_canvas.set_widget(node_view);
        */
        // Brushes
        let mut brushes_rowlist_layout =
            TheRowListLayout::new(TheId::named("ModelFX Brushes List"));

        let tools = BRUSHLIST.lock().unwrap();
        for (index, brush) in tools.brushes.values().enumerate() {
            let mut item = TheRowListItem::new(TheId::named_with_id("Brush", brush.id().uuid));
            item.set_text(brush.info().clone());
            //item.set_icon(material.get_preview());

            if index == 0 {
                item.set_state(TheWidgetState::Selected);
                server_ctx.curr_brush = brush.id().uuid;
            }

            let mut buffer = TheRGBABuffer::new(TheDim::sized(300, 300));
            brush.preview(&mut buffer);
            item.set_icon(buffer);

            brushes_rowlist_layout.add_item(item, _ctx);
        }

        //brushes_canvas.set_layout(brushes_rowlist_layout);

        // modelfx_stack.add_canvas(material_canvas);
        // modelfx_stack.add_canvas(texture_node_canvas);
        // modelfx_stack.add_canvas(brushes_canvas);

        canvas.set_layout(brushes_rowlist_layout);

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
            TheEvent::PaletteIndexChanged(_, index) => {
                if *ACTIVEEDITOR.lock().unwrap() == ActiveEditor::MaterialEditor {
                    if let Some(material_id) = server_ctx.curr_material {
                        if let Some(material) = project.materials.get_mut(&material_id) {
                            if let Some(selected_index) = material.selected_node {
                                let prev = material.to_json();
                                if material.nodes[selected_index].set_palette_index(*index) {
                                    material.render_preview(
                                        &project.palette,
                                        &TILEDRAWER.lock().unwrap().tiles,
                                    );
                                    ui.set_node_preview(
                                        "MaterialFX NodeCanvas",
                                        0,
                                        material.get_preview(),
                                    );
                                    if let Some(list) =
                                        ui.get_rowlist_layout("ModelFX Material List")
                                    {
                                        list.set_item_image(material.id, material.get_preview());
                                    }

                                    let next = material.to_json();

                                    TILEDRAWER
                                        .lock()
                                        .unwrap()
                                        .set_materials(project.materials.clone());

                                    // if let Some(widget) = ui.get_widget(&id.name) {
                                    //     widget.set_value(TheValue::ColorObject(color));
                                    // }

                                    let undo = MaterialFXUndoAtom::Edit(material_id, prev, next);
                                    UNDOMANAGER.lock().unwrap().add_materialfx_undo(undo, ctx);

                                    redraw = true;
                                }
                            }
                        }
                    }
                }
            }
            TheEvent::SizeChanged(id) => {
                if id.name == "GeoFX RGBA Layout" {
                    self.set_geo_tiles(&project.palette, ui, ctx);
                }
            }
            TheEvent::StateChanged(id, state) => {
                if id.name == "Brush" {
                    server_ctx.curr_brush = id.uuid;
                } else if id.name == "Material Object" {
                    if let Some(material) = project.materials.get(&id.uuid) {
                        server_ctx.curr_material = Some(material.id);
                    } else {
                        server_ctx.curr_material = None;
                        ui.set_node_canvas("MaterialFX NodeCanvas", TheNodeCanvas::default());
                    }
                    self.set_current_brush(ui, project, server_ctx, ctx);
                } else if id.name == "MaterialFX Add" {
                    let mut material = MaterialFXObject::default();
                    let node = MaterialFXNode::new(MaterialFXNodeRole::Geometry);
                    material.nodes.push(node);
                    material.selected_node = Some(material.nodes.len() - 1);
                    material.render_preview(&project.palette, &TILEDRAWER.lock().unwrap().tiles);
                    ui.set_node_preview("MaterialFX NodeCanvas", 0, material.get_preview());
                    if let Some(list) = ui.get_rowlist_layout("ModelFX Material List") {
                        list.set_item_image(material.id, material.get_preview());
                    }

                    // if let Some(region) = project.get_region(&server_ctx.curr_region) {
                    //     let area = region.get_material_area(material.id);
                    //     PRERENDERTHREAD.lock().unwrap().render_region(
                    //         region.clone(),
                    //         project.palette.clone(),
                    //         area,
                    //     );
                    // }
                    let material_id = material.id;

                    let undo = MaterialFXUndoAtom::AddMaterial(material.clone());
                    UNDOMANAGER.lock().unwrap().add_materialfx_undo(undo, ctx);

                    project.materials.insert(material.id, material);
                    server_ctx.curr_material = Some(material_id);
                    TILEDRAWER
                        .lock()
                        .unwrap()
                        .set_materials(project.materials.clone());
                } else if id.name == "ModelFX Clear" && state == &TheWidgetState::Clicked {
                    // self.modelfx = ModelFX::default();
                    // self.modelfx.draw(ui, ctx, &project.palette);
                    // self.render_preview(ui, &project.palette);
                    redraw = true;
                } else if id.name == "ModelFX Move" && state == &TheWidgetState::Clicked {
                    // if !self.modelfx.nodes.is_empty() {
                    //     project.models.push(self.modelfx.clone());
                    //     ctx.ui.send(TheEvent::Custom(
                    //         TheId::named("ModelFX Redraw Library"),
                    //         TheValue::Empty,
                    //     ));
                    //     self.redraw_modelfx_library(project, ui, ctx);
                    // }
                    redraw = true;
                }
            }
            TheEvent::ContextMenuSelected(id, item) => {
                //let prev = self.modelfx.to_json();
                #[allow(clippy::collapsible_if)]
                if id.name == "MaterialFX Nodes" || id.name.is_empty() {
                    if let Some(material_id) = server_ctx.curr_material {
                        if let Some(material) = project.materials.get_mut(&material_id) {
                            let prev = material.to_json();
                            let mut node = MaterialFXNode::new_from_name(item.name.clone());
                            node.position = vec2i(
                                material.scroll_offset.x + 220,
                                material.scroll_offset.y + 10,
                            );
                            let index = material.nodes.len();
                            if index > 0 && node.supports_preview {
                                node.render_preview(&project.palette);
                            }
                            material.nodes.push(node);
                            material.selected_node = Some(material.nodes.len() - 1);
                            let undo =
                                MaterialFXUndoAtom::AddNode(material.id, prev, material.to_json());
                            UNDOMANAGER.lock().unwrap().add_materialfx_undo(undo, ctx);
                            let node_canvas = material.to_canvas(&project.palette);
                            ui.set_node_canvas("MaterialFX NodeCanvas", node_canvas);
                            self.set_selected_material_node_ui(server_ctx, project, ui, ctx, true);
                        }
                    }
                    redraw = true;
                }
            }
            TheEvent::TilePicked(id, coord) => {
                if id.name == "GeoFX RGBA Layout View" {
                    server_ctx.curr_geo_object = None;
                    server_ctx.curr_geo_node = None;
                    self.set_geo_node_ui(server_ctx, project, ui, ctx);
                    self.set_current_brush(ui, project, server_ctx, ctx);
                } else if id.name == "MaterialFX RGBA Layout View" {
                    if let Some(material) = self.materials.get(&(coord.x, coord.y)) {
                        server_ctx.curr_material = Some(*material);
                    } else {
                        server_ctx.curr_material = None;
                        ui.set_node_canvas("MaterialFX NodeCanvas", TheNodeCanvas::default());
                    }
                    self.set_current_brush(ui, project, server_ctx, ctx);
                    redraw = true;
                }
            }
            TheEvent::TileEditorClicked(id, _coord) => {
                if id.name == "GeoFX RGBA Layout View" {
                    //&& self.modelfx.clicked(*coord, ui, ctx) {
                    //self.modelfx.draw(ui, ctx, &project.palette);
                    self.set_selected_material_node_ui(server_ctx, project, ui, ctx, true);
                    //self.render_preview(ui, &project.palette);
                    redraw = true;
                }
            }
            TheEvent::TileEditorDragged(id, _coord) => {
                if id.name == "GeoFX RGBA Layout View" {
                    //&& self.modelfx.dragged(*coord, ui, ctx) {
                    //self.modelfx.draw(ui, ctx, &project.palette);
                    redraw = true;
                }
            }
            TheEvent::TileEditorUp(id) => {
                //let prev = self.modelfx.to_json();
                if id.name == "GeoFX RGBA Layout View" {
                    //&& self.modelfx.released(ui, ctx) {
                    //self.modelfx.draw(ui, ctx, &project.palette);
                    //self.render_preview(ui, &project.palette);
                    //let undo = ModelFXUndoAtom::Edit(prev, self.modelfx.to_json());
                    //UNDOMANAGER.lock().unwrap().add_modelfx_undo(undo, ctx);
                    redraw = true;
                }
            }
            TheEvent::TileEditorHoverChanged(id, coord) => {
                if id.name == "GeoFX RGBA Layout View" {
                    ctx.ui.send(TheEvent::SetStatusText(
                        id.clone(),
                        self.geos_description
                            .get(&(coord.x, coord.y))
                            .unwrap_or(&"".to_string())
                            .to_string(),
                    ));
                    redraw = true;
                }
            }
            TheEvent::TileEditorDelete(_id, _selected) => {
                // if id.name == "ModelFX Library RGBA Layout View" {
                //     let prev = self.modelfx.to_json();
                //     for pos in selected {
                //         let index = (pos.0 + pos.1 * 4) as usize;
                //         project.models.remove(index);
                //     }
                //     self.redraw_modelfx_library(project, ui, ctx);
                //     let undo = MaterialFXUndoAtom::Edit(prev, self.modelfx.to_json());
                //     UNDOMANAGER.lock().unwrap().add_materialfx_undo(undo, ctx);
                //     redraw = true;
                // } else if id.name == "GeoFX RGBA Layout View" {
                //     let prev = self.modelfx.to_json();
                //     self.modelfx.delete();
                //     self.modelfx.draw(ui, ctx, &project.palette);
                //     self.render_preview(ui, &project.palette);
                //     let undo = MaterialFXUndoAtom::Edit(prev, self.modelfx.to_json());
                //     UNDOMANAGER.lock().unwrap().add_materialfx_undo(undo, ctx);
                //     redraw = true;
                // }
            }
            TheEvent::ColorButtonClicked(id) => {
                // When a color button is clicked, copy over the current palette index.
                if id.name.starts_with(":MODELFX:") {
                    //let prev = self.modelfx.to_json();
                    if let Some(name) = id.name.strip_prefix(":MODELFX: ") {
                        if let Some(material_id) = server_ctx.curr_material {
                            if let Some(material) = project.materials.get_mut(&material_id) {
                                if let Some(selected_index) = material.selected_node {
                                    if let Some(color) = project.palette.get_current_color() {
                                        let mut old_index = None;
                                        if let Some(TheValue::PaletteIndex(index)) =
                                            material.nodes[selected_index].get(name)
                                        {
                                            old_index = Some(index);
                                        }

                                        let prev = material.to_json();
                                        material.nodes[selected_index].set(
                                            name,
                                            TheValue::PaletteIndex(project.palette.current_index),
                                        );

                                        material.render_preview(
                                            &project.palette,
                                            &TILEDRAWER.lock().unwrap().tiles,
                                        );
                                        ui.set_node_preview(
                                            "MaterialFX NodeCanvas",
                                            0,
                                            material.get_preview(),
                                        );
                                        if let Some(list) =
                                            ui.get_rowlist_layout("ModelFX Material List")
                                        {
                                            list.set_item_image(
                                                material.id,
                                                material.get_preview(),
                                            );
                                        }

                                        let next = material.to_json();

                                        TILEDRAWER
                                            .lock()
                                            .unwrap()
                                            .set_materials(project.materials.clone());

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

                                        let undo =
                                            MaterialFXUndoAtom::Edit(material_id, prev, next);
                                        UNDOMANAGER.lock().unwrap().add_materialfx_undo(undo, ctx);

                                        redraw = true;
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
                if id.name == "Brush Size" {
                    if let Some(size) = value.to_f32() {
                        self.brush_size = size;
                    }
                } else if id.name == "Falloff" {
                    if let Some(size) = value.to_f32() {
                        self.falloff = size;
                    }
                } else if id.name == "ModelFX Blend" {
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
                } else if id.name.starts_with(":MODELFX:") {
                    if let Some(name) = id.name.strip_prefix(":MODELFX: ") {
                        let mut value = value.clone();

                        if self.geometry_mode {
                            if let Some(curr_geo_node) = server_ctx.curr_geo_node {
                                let mut old_tiles_to_render: Vec<Vec2i> = vec![];
                                let mut new_tiles_to_render: Vec<Vec2i> = vec![];
                                let palette = project.palette.clone();

                                if let Some(region) =
                                    project.get_region_mut(&server_ctx.curr_region)
                                {
                                    if let Some((geo_obj, index)) =
                                        region.find_geo_node(curr_geo_node)
                                    {
                                        old_tiles_to_render.clone_from(&geo_obj.area);

                                        // Convert TextList back
                                        let coll = geo_obj.nodes[index].collection();
                                        if let Some(TheValue::TextList(_, list)) = coll.get(name) {
                                            if let Some(v) = value.to_i32() {
                                                value = TheValue::TextList(v, list.clone());
                                            }
                                        }

                                        geo_obj.nodes[index].set(name, value.clone());
                                        match &geo_obj.nodes[index].role {
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
                                        let geo_obj_id = geo_obj.id;

                                        new_tiles_to_render.clone_from(&geo_obj.area);
                                        let mut set: FxHashSet<Vec2i> = FxHashSet::default();
                                        set.extend(&old_tiles_to_render);
                                        set.extend(&new_tiles_to_render);

                                        region.update_geometry_areas();
                                        server.update_region(region);
                                        region.compile_geo(
                                            geo_obj_id,
                                            &palette,
                                            &TILEDRAWER.lock().unwrap().tiles,
                                        );
                                    }
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
                        } else {
                            #[allow(clippy::collapsible_else_if)]
                            if let Some(material_id) = server_ctx.curr_material {
                                if let Some(material) = project.materials.get_mut(&material_id) {
                                    if let Some(selected_index) = material.selected_node {
                                        let prev = material.to_json();

                                        // Convert TextList back
                                        if let Some(TheValue::TextList(_, list)) =
                                            material.nodes[selected_index].get(name)
                                        {
                                            if let Some(v) = value.to_i32() {
                                                value = TheValue::TextList(v, list.clone());
                                            }
                                        }

                                        // Look up the texture.
                                        if material.nodes[selected_index].role
                                            == MaterialFXNodeRole::Material
                                        {
                                            if let TheValue::Text(tags) = &value {
                                                if let Some(TheValue::Tile(_, id)) = TILEDRAWER
                                                    .lock()
                                                    .unwrap()
                                                    .get_tile_by_tags(0, &tags.to_lowercase())
                                                {
                                                    material.nodes[selected_index].texture_id =
                                                        Some(id);
                                                } else {
                                                    material.nodes[selected_index].texture_id =
                                                        None;
                                                }
                                            }
                                        }

                                        material.nodes[selected_index].set(name, value);

                                        if material.nodes[selected_index].supports_preview {
                                            material.nodes[selected_index]
                                                .render_preview(&project.palette);
                                            ui.set_node_preview(
                                                "MaterialFX NodeCanvas",
                                                selected_index,
                                                material.nodes[selected_index].preview.clone(),
                                            );
                                        }
                                        material.render_preview(
                                            &project.palette,
                                            &TILEDRAWER.lock().unwrap().tiles,
                                        );
                                        ui.set_node_preview(
                                            "MaterialFX NodeCanvas",
                                            0,
                                            material.get_preview(),
                                        );
                                        let next = material.to_json();
                                        TILEDRAWER
                                            .lock()
                                            .unwrap()
                                            .set_materials(project.materials.clone());

                                        let undo =
                                            MaterialFXUndoAtom::Edit(material_id, prev, next);
                                        UNDOMANAGER.lock().unwrap().add_materialfx_undo(undo, ctx);
                                        self.render_material_changes(
                                            material_id,
                                            server_ctx,
                                            project,
                                            ui,
                                        );
                                    }
                                }
                            }
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
                if id.name == "Update Materials" {
                    println!("Update Materials");
                } else if id.name == "Floor Selected" {
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
                        // if *index == 0 {
                        //     self.editing_mode = EditingMode::Geometry;
                        //     self.set_geo_node_ui(server_ctx, project, ui, ctx);
                        // } else
                        if *index == 0 {
                            self.material_mode = MaterialMode::Material;
                        } else {
                            self.material_mode = MaterialMode::MaterialNodes;
                            self.set_selected_material_node_ui(server_ctx, project, ui, ctx, true);
                        }
                    }
                }
            }
            TheEvent::NodeSelectedIndexChanged(id, index) => {
                if id.name == "MaterialFX NodeCanvas" {
                    if let Some(material_id) = server_ctx.curr_material {
                        if let Some(material) = project.materials.get_mut(&material_id) {
                            material.selected_node = *index;
                        }
                    }
                    self.set_selected_material_node_ui(server_ctx, project, ui, ctx, true);
                }
            }
            TheEvent::NodeDragged(id, index, position) => {
                if id.name == "MaterialFX NodeCanvas" {
                    if let Some(material_id) = server_ctx.curr_material {
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
                            let prev = material.to_json();
                            material.connections.clone_from(connections);
                            material.render_preview(
                                &project.palette,
                                &TILEDRAWER.lock().unwrap().tiles,
                            );
                            ui.set_node_preview("MaterialFX NodeCanvas", 0, material.get_preview());
                            if let Some(list) = ui.get_rowlist_layout("ModelFX Material List") {
                                list.set_item_image(material.id, material.get_preview());
                            }
                            let undo =
                                MaterialFXUndoAtom::Edit(material.id, prev, material.to_json());
                            UNDOMANAGER.lock().unwrap().add_materialfx_undo(undo, ctx);
                            redraw = true;
                        }
                        self.render_material_changes(material_id, server_ctx, project, ui);
                    }
                }
            }
            TheEvent::NodeDeleted(id, deleted_node_index, connections) => {
                if id.name == "MaterialFX NodeCanvas" {
                    if let Some(material_id) = server_ctx.curr_material_object {
                        if let Some(material) = project.materials.get_mut(&material_id) {
                            let prev = material.to_json();
                            material.nodes.remove(*deleted_node_index);
                            //material.node_previews.remove(*deleted_node_index);
                            material.connections.clone_from(connections);
                            material.selected_node = None;
                            material.render_preview(
                                &project.palette,
                                &TILEDRAWER.lock().unwrap().tiles,
                            );
                            let preview = material.get_preview();
                            ui.set_node_preview("MaterialFX NodeCanvas", 0, preview.clone());
                            let undo =
                                MaterialFXUndoAtom::Edit(material.id, prev, material.to_json());
                            UNDOMANAGER.lock().unwrap().add_materialfx_undo(undo, ctx);
                            redraw = true;
                        }
                        self.render_material_changes(material_id, server_ctx, project, ui);
                    }
                }
            }
            TheEvent::NodeViewScrolled(id, offset) => {
                if id.name == "MaterialFX NodeCanvas" {
                    if let Some(material_id) = server_ctx.curr_material_object {
                        if let Some(material) = project.materials.get_mut(&material_id) {
                            material.scroll_offset = *offset;
                        }
                    }
                }
            }

            _ => {}
        }

        redraw
    }

    /// Set the preview for the current geometry or material we are painting with
    pub fn set_current_brush(
        &self,
        ui: &mut TheUI,
        project: &Project,
        server_ctx: &mut ServerContext,
        ctx: &mut TheContext,
    ) {
        if self.geometry_mode {
            if let Some(curr_geo_node) = self.get_geo_node(ui) {
                let mut buffer = TheRGBABuffer::new(TheDim::sized(65, 65));
                curr_geo_node.preview(
                    &mut buffer,
                    None,
                    &project.palette,
                    &TILEDRAWER.lock().unwrap().tiles,
                    Vec2f::zero(),
                    ctx,
                );

                if let Some(icon_view) = ui.get_icon_view("Icon Preview") {
                    icon_view.set_rgba_tile(TheRGBATile::buffer(buffer));
                }
            }
        } else if let Some(curr_material) = server_ctx.curr_material_object {
            if let Some(material) = project.materials.get(&curr_material) {
                if let Some(icon_view) = ui.get_icon_view("Icon Preview") {
                    icon_view.set_rgba_tile(TheRGBATile::buffer(material.get_preview()));
                }
            }
        }
    }

    pub fn set_geometry_mode(&mut self, geometry_mode: bool) {
        self.geometry_mode = geometry_mode;
    }

    /// Modeler got activated, set the UI
    pub fn activated(
        &mut self,
        server_ctx: &mut ServerContext,
        project: &mut Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) {
        self.set_current_brush(ui, project, server_ctx, ctx);
        if self.geometry_mode {
            self.set_geo_node_ui(server_ctx, project, ui, ctx);
        } else if self.material_mode == MaterialMode::Material {
        } else {
            self.set_selected_material_node_ui(server_ctx, project, ui, ctx, true);
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
        //let mut node_name = String::new();

        if let Some(curr_geo_node) = server_ctx.curr_geo_node {
            if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                if let Some((geo_obj, index)) = region.find_geo_node(curr_geo_node) {
                    collection = Some(geo_obj.nodes[index].collection());
                }
            }
            //node_name = curr_geo_node.d
        } else if let Some(geo) = self.get_geo_node(ui) {
            collection = Some(geo.collection());
        }

        if let Some(collection) = collection {
            if let Some(text_layout) = ui.get_text_layout("Node Settings") {
                text_layout.clear();

                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Show Node Settings"),
                    TheValue::Text("Geometry Node".to_string()),
                ));

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
        } else if let Some(text_layout) = ui.get_text_layout("Node Settings") {
            text_layout.clear();
        }
    }

    pub fn set_material_node_ui(
        &mut self,
        server_ctx: &mut ServerContext,
        project: &mut Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        switch_to_nodes: bool,
    ) {
        if let Some(text_layout) = ui.get_text_layout("Node Settings") {
            text_layout.clear();

            if switch_to_nodes {
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Show Node Settings"),
                    TheValue::Text("Material".to_string()),
                ));
            }

            // let mut add_button = TheTraybarButton::new(TheId::named("MaterialFX Add"));
            // add_button.set_text("New Material".to_string());
            // add_button.set_status_text("Add a new material");

            // text_layout.add_pair("Add".to_string(), Box::new(add_button));

            if let Some(materialid) = server_ctx.curr_material_object {
                if let Some(material) = project.materials.get_mut(&materialid) {
                    let node_canvas = material.to_canvas(&project.palette);
                    ui.set_node_canvas("MaterialFX NodeCanvas", node_canvas);
                }
            }
        }
    }

    pub fn set_selected_material_node_ui(
        &mut self,
        server_ctx: &mut ServerContext,
        project: &mut Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        switch_to_nodes: bool,
    ) {
        self.palette_indices.clear();

        if let Some(material_id) = server_ctx.curr_material_object {
            if let Some(material) = project.materials.get_mut(&material_id) {
                if let Some(selected_index) = material.selected_node {
                    // Safeguard, not actually needed
                    if selected_index >= material.nodes.len() {
                        material.selected_node = None;
                        return;
                    }

                    let collection = material.nodes[selected_index].collection();

                    if let Some(text_layout) = ui.get_text_layout("Node Settings") {
                        text_layout.clear();

                        if switch_to_nodes {
                            ctx.ui.send(TheEvent::Custom(
                                TheId::named("Show Node Settings"),
                                TheValue::Text("Material Node".to_string()),
                            ));
                        }

                        for (name, value) in &collection.keys {
                            if let TheValue::Text(text) = value {
                                let mut edit = TheTextLineEdit::new(TheId::named(
                                    (":MODELFX: ".to_owned() + name).as_str(),
                                ));
                                edit.set_value(TheValue::Text(text.clone()));
                                text_layout.add_pair(name.clone(), Box::new(edit));
                            } else if let TheValue::FloatRange(value, range) = value {
                                let mut slider = TheTextLineEdit::new(TheId::named(
                                    (":MODELFX: ".to_owned() + name).as_str(),
                                ));
                                slider.set_value(TheValue::Float(*value));
                                //slider.set_default_value(TheValue::Float(0.0));
                                slider.set_range(TheValue::RangeF32(range.clone()));
                                //slider.set_continuous(true);
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
        } else if let Some(text_layout) = ui.get_text_layout("Node Settings") {
            text_layout.clear();
        }
    }

    pub fn set_curr_layer_role(
        &mut self,
        layer_role: Layer2DRole,
        palette: &ThePalette,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) {
        self.curr_layer_role = layer_role;
        self.set_geo_tiles(palette, ui, ctx);
    }

    /// Set the tiles for the picker.
    pub fn set_geo_tiles(&mut self, _palette: &ThePalette, ui: &mut TheUI, ctx: &mut TheContext) {
        let tile_size = 65;

        //let mut set_default_selection = false;

        let geo_tiles = GeoFXNode::nodes();
        self.geos_description.clear();

        self.geos.clear();
        let mut amount = 0;
        for g in geo_tiles.iter() {
            if g.get_layer_role() == self.curr_layer_role {
                amount += 1;
            }
        }

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
                let lines = amount / tiles_per_row + 1;

                let mut buffer =
                    TheRGBABuffer::new(TheDim::sized(width, max(lines * grid, height)));

                let mut tile_buffer = TheRGBABuffer::new(TheDim::sized(tile_size, tile_size));

                let mut i = 0;
                for tile in geo_tiles.iter() {
                    if tile.get_layer_role() != self.curr_layer_role {
                        continue;
                    }

                    let x = i % tiles_per_row;
                    let y = i / tiles_per_row;

                    self.geos_description.insert((x, y), tile.description());

                    i += 1;

                    if let Some(icon_name) = tile.icon_name() {
                        if let Some(b) = ctx.ui.icon(&icon_name) {
                            tile_buffer.copy_into(0, 0, b);
                        } else {
                            tile_buffer.fill(BLACK);
                        }
                    } else {
                        tile_buffer.fill(BLACK);
                    }

                    // tile.preview(
                    //     &mut tile_buffer,
                    //     None,
                    //     palette,
                    //     &FxHashMap::default(),
                    //     Vec2f::zero(),
                    //     ctx,
                    // );
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

    /// Global events to change the material and its previews.
    pub fn render_material_changes(
        &mut self,
        material_id: Uuid,
        _server_ctx: &mut ServerContext,
        project: &mut Project,
        ui: &mut TheUI,
    ) {
        if let Some(material) = project.materials.get_mut(&material_id) {
            if let Some(layout) = ui.get_list_layout("Material List") {
                let preview = material.get_preview();
                layout.set_item_icon(material_id, preview.scaled(36, 36));
            }
        }

        TILEDRAWER
            .lock()
            .unwrap()
            .set_materials(project.materials.clone());
    }

    /// Renders the preview of the material.
    pub fn render_material_preview(&mut self, _material_id: Uuid, _project: &mut Project) {}
}
