use crate::editor::RUSTERIX;
use crate::prelude::*;
use ShapeFXParam::*;
use rusterix::ShapeFXParam;
use rusterix::{Light, LightType, Shape, ShapeType, TileRole};
use theframework::prelude::*;

pub struct ShapePicker {
    pub id: String,

    pub shape_map: FxHashMap<(i32, i32), ShapeType>,
    pub shape_text: FxHashMap<(i32, i32), String>,

    pub filter: String,
    pub filter_role: u8,
    pub zoom: f32,

    pub effects: Vec<EffectWrapper>,

    pub curr_shape_type: Option<ShapeType>,

    pub shapes: FxHashMap<ShapeType, Shape>,
}

#[allow(clippy::new_without_default)]
impl ShapePicker {
    pub fn new(id: String) -> Self {
        let effects = vec![
            EffectWrapper::RusterixLight(Light::new(LightType::Point)),
            EffectWrapper::RusterixLight(Light::new(LightType::Area)),
            EffectWrapper::RusterixLight(Light::new(LightType::Daylight)),
        ];

        let mut shapes: FxHashMap<ShapeType, Shape> = FxHashMap::default();
        shapes.insert(ShapeType::Circle, Shape::new(ShapeType::Circle));
        shapes.insert(ShapeType::Star, Shape::new(ShapeType::Star));
        shapes.insert(ShapeType::Bricks, Shape::new(ShapeType::Bricks));

        Self {
            id,
            shape_map: FxHashMap::default(),
            shape_text: FxHashMap::default(),
            filter: "".to_string(),
            filter_role: 0,
            zoom: 1.0,

            effects,
            shapes,

            curr_shape_type: None,
        }
    }

    /// Build the tile picker UI
    pub fn build(&self, minimal: bool) -> TheCanvas {
        let mut canvas = TheCanvas::new();

        // Toolbar
        let mut toolbar_canvas = TheCanvas::default();
        let traybar_widget = TheTraybar::new(TheId::empty());
        toolbar_canvas.set_widget(traybar_widget);
        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.set_background_color(None);

        let mut filter_text = TheText::new(TheId::empty());
        filter_text.set_text(fl!("filter"));

        toolbar_hlayout.set_margin(Vec4::new(10, 1, 5, 1));
        toolbar_hlayout.set_padding(3);
        toolbar_hlayout.add_widget(Box::new(filter_text));
        let mut filter_edit = TheTextLineEdit::new(TheId::named(&self.make_id(" Filter Edit")));
        filter_edit.set_text("".to_string());
        filter_edit
            .limiter_mut()
            .set_max_size(Vec2::new(if minimal { 75 } else { 120 }, 18));
        filter_edit.set_font_size(12.5);
        filter_edit.set_embedded(true);
        filter_edit.set_status_text(&fl!("status_shape_picker_filter_edit"));
        filter_edit.set_continuous(true);
        toolbar_hlayout.add_widget(Box::new(filter_edit));

        if !minimal {
            let mut spacer = TheSpacer::new(TheId::empty());
            spacer.limiter_mut().set_max_width(10);
            toolbar_hlayout.add_widget(Box::new(spacer));
        }

        let mut drop_down = TheDropdownMenu::new(TheId::named(&self.make_id(" Filter Role")));
        drop_down.add_option(fl!("all"));
        for dir in TileRole::iterator() {
            drop_down.add_option(dir.to_string().to_string());
        }
        toolbar_hlayout.add_widget(Box::new(drop_down));

        if !minimal {
            let mut zoom = TheSlider::new(TheId::named(&self.make_id(" Zoom")));
            zoom.set_value(TheValue::Float(self.zoom));
            zoom.set_default_value(TheValue::Float(1.5));
            zoom.set_range(TheValue::RangeF32(1.0..=3.0));
            zoom.set_continuous(true);
            zoom.limiter_mut().set_max_width(120);
            toolbar_hlayout.add_widget(Box::new(zoom));
            toolbar_hlayout.set_reverse_index(Some(1));
        }

        toolbar_canvas.set_layout(toolbar_hlayout);

        // Canvas
        let mut rgba_layout = TheRGBALayout::new(TheId::named(&self.make_id(" RGBA Layout")));
        if let Some(rgba_view) = rgba_layout.rgba_view_mut().as_rgba_view() {
            rgba_view.set_grid(Some(48));
            rgba_view.set_mode(TheRGBAViewMode::TilePicker);
            let c = [200, 200, 200, 200];
            rgba_view.set_hover_color(Some(c));
            rgba_view.set_selection_color(c);
            rgba_view.set_icon_mode(true);
        }

        canvas.set_layout(rgba_layout);
        canvas
    }

    /// Set the tiles for the picker.
    pub fn update_tiles(&mut self, _project: &Project, ui: &mut TheUI, ctx: &mut TheContext) {
        self.shape_map.clear();
        self.shape_text.clear();
        if let Some(editor) = ui.get_rgba_layout(&self.make_id(" RGBA Layout")) {
            let width = editor.dim().width - 16;
            let height = editor.dim().height - 16;

            if width == -16 {
                return;
            }

            if let Some(rgba_view) = editor.rgba_view_mut().as_rgba_view() {
                let grid = (48_f32 * self.zoom) as i32;
                rgba_view.set_grid(Some(grid));

                let mut filtered_tiles = vec![];

                for effect in &self.effects {
                    if effect.name().to_lowercase().contains(&self.filter)
                    //&& (self.filter_role == 0 || map.role == self.filter_role - 1)
                    {
                        filtered_tiles.push(effect.clone());
                    }
                }

                let tiles_per_row = width / grid;
                let lines = filtered_tiles.len() as i32 / tiles_per_row + 1;

                let mut buffer =
                    TheRGBABuffer::new(TheDim::sized(width, (lines * grid).max(height)));

                for (i, _) in filtered_tiles.iter().enumerate() {
                    let x = i as i32 % tiles_per_row;
                    let y = i as i32 / tiles_per_row;

                    let mut rgba = TheRGBABuffer::from(
                        vec![0_u8; grid as usize * grid as usize * 4],
                        grid as u32,
                        grid as u32,
                    );

                    let shape_type = ShapeType::from(i as i32);
                    let mut shape = Shape::new(shape_type);
                    shape.preview(&mut rgba, &RUSTERIX.read().unwrap().assets);
                    self.shape_map.insert((x, y), shape_type);
                    self.shape_text.insert((x, y), shape.shape_type.to_string());

                    buffer.copy_into(x * grid, y * grid, &rgba);
                }

                rgba_view.set_buffer(buffer);
            }
            editor.relayout(ctx);
        }
    }

    pub fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let mut redraw = false;

        match event {
            TheEvent::Resize => {
                self.update_tiles(project, ui, ctx);
            }
            TheEvent::TilePicked(id, pos) => {
                if id.name == self.make_id(" RGBA Layout View") {
                    if let Some(shape_type) = self.shape_map.get(&(pos.x, pos.y)) {
                        self.curr_shape_type = Some(*shape_type);
                        self.apply_shape_settings(ui, ctx, project, server_ctx);
                        redraw = true;
                        self.activate_shape_paste(server_ctx);
                    }
                }
            }
            TheEvent::TileEditorHoverChanged(id, pos) => {
                if id.name == self.make_id(" RGBA Layout View") {
                    ctx.ui.send(TheEvent::SetStatusText(
                        id.clone(),
                        self.shape_text
                            .get(&(pos.x, pos.y))
                            .unwrap_or(&"".to_string())
                            .to_string(),
                    ));
                }
            }
            TheEvent::Custom(id, _value) => {
                if id.name == "Update Materialpicker" {
                    self.update_tiles(project, ui, ctx);
                }
            }
            // TheEvent::StateChanged(id, state) => {
            //     if id.name == self.make_id(" Tile Details") && *state == TheWidgetState::Clicked {
            //         if let Some(layout) = ui.get_layout(" Tile Details Layout") {
            //             if layout.limiter().get_max_width() == 0 {
            //                 layout.limiter_mut().set_max_width(150);
            //             } else {
            //                 layout.limiter_mut().set_max_width(0);
            //             }
            //             ctx.ui.relayout = true;
            //         }
            //         ctx.ui.send(TheEvent::Custom(
            //             TheId::named("Update Tilepicker"),
            //             TheValue::Empty,
            //         ));
            //     }
            //}
            TheEvent::ValueChanged(id, value) => {
                if id.name.starts_with("shapepicker") {
                    let snake_case = self.transform_to_snake_case(&id.name, "shapepicker");
                    if let Some(shape_type) = self.curr_shape_type {
                        if let Some(node) = self.shapes.get_mut(&shape_type) {
                            match value {
                                TheValue::FloatRange(v, _) => {
                                    node.values.set(&snake_case, rusterix::Value::Float(*v))
                                }
                                TheValue::IntRange(v, _) => {
                                    node.values.set(&snake_case, rusterix::Value::Int(*v))
                                }
                                TheValue::Int(v) => {
                                    node.values.set(&snake_case, rusterix::Value::Int(*v))
                                }
                                TheValue::ColorObject(v) => node
                                    .values
                                    .set(&snake_case, rusterix::Value::Color(v.clone())),
                                _ => {}
                            }
                        }
                        self.activate_shape_paste(server_ctx);
                        self.update_tiles(project, ui, ctx);
                    }
                }
            }
            _ => {}
        }
        redraw
    }

    /// Activates the current shape for pasting
    pub fn activate_shape_paste(&self, server_ctx: &mut ServerContext) {
        if let Some(shape_type) = self.curr_shape_type {
            if let Some(shape) = self.shapes.get(&shape_type) {
                let mut map = Map {
                    subdivisions: 1000.0,
                    ..Default::default()
                };
                shape.create(&mut map, None, None);
                server_ctx.paste_clipboard = Some(map);
            }
        }
    }

    /// Sets the node settings for the map selection.
    fn apply_shape_settings(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        _server_ctx: &mut ServerContext,
    ) {
        // Create Node Settings if necessary
        if let Some(layout) = ui.get_text_layout("Node Settings") {
            layout.clear();
        }

        let mut nodeui = TheNodeUI::default();

        if let Some(shape_type) = self.curr_shape_type {
            if let Some(node) = self.shapes.get(&shape_type) {
                for param in node.params() {
                    match param {
                        Float(id, name, status, value, range) => {
                            let item = TheNodeUIItem::FloatEditSlider(
                                format!(
                                    "shapepicker{}",
                                    id.get(0..1).unwrap_or("").to_uppercase()
                                        + id.get(1..).unwrap_or("")
                                ),
                                name.clone(),
                                status.clone(),
                                value,
                                range,
                                false,
                            );
                            nodeui.add_item(item);
                        }
                        Int(id, name, status, value, range) => {
                            let item = TheNodeUIItem::IntEditSlider(
                                format!(
                                    "shapepicker{}",
                                    id.get(0..1).unwrap_or("").to_uppercase()
                                        + id.get(1..).unwrap_or("")
                                ),
                                name.clone(),
                                status.clone(),
                                value,
                                range,
                                false,
                            );
                            nodeui.add_item(item);
                        }
                        Color(id, name, status, value) => {
                            let item = TheNodeUIItem::ColorPicker(
                                format!(
                                    "shapepicker{}",
                                    id.get(0..1).unwrap_or("").to_uppercase()
                                        + id.get(1..).unwrap_or("")
                                ),
                                name.clone(),
                                status.clone(),
                                value,
                                false,
                            );
                            nodeui.add_item(item);
                        }
                        PaletteIndex(id, name, status, value) => {
                            let item = TheNodeUIItem::PaletteSlider(
                                format!(
                                    "shapepicker{}",
                                    id.get(0..1).unwrap_or("").to_uppercase()
                                        + id.get(1..).unwrap_or("")
                                ),
                                name.clone(),
                                status.clone(),
                                value,
                                project.palette.clone(),
                                false,
                            );
                            nodeui.add_item(item);
                        }
                        Selector(id, name, status, options, value) => {
                            let item = TheNodeUIItem::Selector(
                                format!(
                                    "shapepicker{}",
                                    id.get(0..1).unwrap_or("").to_uppercase()
                                        + id.get(1..).unwrap_or("")
                                ),
                                name.clone(),
                                status.clone(),
                                options.clone(),
                                value,
                            );
                            nodeui.add_item(item);
                        }
                    }
                }
            }
        }

        if let Some(layout) = ui.get_text_layout("Node Settings") {
            nodeui.apply_to_text_layout(layout);
            // layout.relayout(ctx);
            ctx.ui.relayout = true;

            ctx.ui.send(TheEvent::Custom(
                TheId::named("Show Node Settings"),
                TheValue::Text("Shape Settings".to_string()),
            ));
        }
    }

    fn transform_to_snake_case(&self, input: &str, strip_prefix: &str) -> String {
        // Strip the prefix if it exists
        let stripped = if let Some(remainder) = input.strip_prefix(strip_prefix) {
            remainder
        } else {
            input
        };

        // Convert to snake_case
        let mut snake_case = String::new();
        for (i, c) in stripped.chars().enumerate() {
            if c.is_uppercase() {
                // Add an underscore before uppercase letters (except for the first character)
                if i > 0 {
                    snake_case.push('_');
                }
                snake_case.push(c.to_ascii_lowercase());
            } else {
                snake_case.push(c);
            }
        }

        snake_case
    }

    ///  Create an id.
    fn make_id(&self, id: &str) -> String {
        self.id.to_owned() + id
    }
}
