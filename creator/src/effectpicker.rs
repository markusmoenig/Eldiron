use crate::prelude::*;
use rusterix::{Light, LightType, TileRole};
use theframework::prelude::*;

pub struct EffectPicker {
    pub id: String,

    pub effects_map: FxHashMap<(i32, i32), EffectWrapper>,
    pub effects_text: FxHashMap<(i32, i32), String>,

    pub filter: String,
    pub filter_role: u8,
    pub zoom: f32,

    pub effects: Vec<EffectWrapper>,

    pub curr_material: Option<Uuid>,
}

#[allow(clippy::new_without_default)]
impl EffectPicker {
    pub fn new(id: String) -> Self {
        let effects = vec![
            EffectWrapper::RusterixLight(Light::new(LightType::Point)),
            EffectWrapper::RusterixLight(Light::new(LightType::Area)),
            EffectWrapper::RusterixLight(Light::new(LightType::Daylight)),
        ];

        Self {
            id,
            effects_map: FxHashMap::default(),
            effects_text: FxHashMap::default(),
            filter: "".to_string(),
            filter_role: 0,
            zoom: 1.0,

            effects,

            curr_material: None,
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
        filter_edit.set_status_text(&fl!("status_effect_picker_filter_edit"));
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
        self.effects_map.clear();
        self.effects_text.clear();
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

                for (i, fx) in filtered_tiles.iter().enumerate() {
                    let x = i as i32 % tiles_per_row;
                    let y = i as i32 / tiles_per_row;

                    let mut rgba = TheRGBABuffer::from(
                        vec![0_u8; grid as usize * grid as usize * 4],
                        grid as u32,
                        grid as u32,
                    );

                    if let Some(icon) = ctx.ui.icon(&fx.icon()) {
                        rgba.copy_into(0, 0, icon);
                    }
                    self.effects_map.insert((x, y), fx.clone());
                    self.effects_text.insert((x, y), fx.name().clone());

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
                    if let Some(tile_id) = self.effects_map.get(&(pos.x, pos.y)) {
                        server_ctx.curr_effect = Some(tile_id.clone());
                        redraw = true;
                    }
                }
            }
            TheEvent::TileEditorHoverChanged(id, pos) => {
                if id.name == self.make_id(" RGBA Layout View") {
                    ctx.ui.send(TheEvent::SetStatusText(
                        id.clone(),
                        self.effects_text
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
                if id.name == self.make_id(" Tile Role") {
                    if let Some(tile_id) = self.curr_material {
                        if let Some(tile) = project.get_tile_mut(&tile_id) {
                            if let TheValue::Int(role) = value {
                                tile.role = TileRole::from_index(*role as u8);
                            }
                        }
                    }
                } else if id.name == self.make_id(" Tile Tags") {
                    if let Some(tile_id) = self.curr_material {
                        if let Some(tile) = project.get_tile_mut(&tile_id) {
                            if let TheValue::Text(tags) = value {
                                tile.name.clone_from(tags);
                            }
                        }
                    }
                } else if id.name == self.make_id(" Filter Edit") {
                    if let TheValue::Text(filter) = value {
                        self.filter = filter.to_lowercase();
                        self.update_tiles(project, ui, ctx);
                    }
                } else if id.name == self.make_id(" Filter Role") {
                    if let TheValue::Int(filter) = value {
                        self.filter_role = *filter as u8;
                        self.update_tiles(project, ui, ctx);
                    }
                } else if id.name == self.make_id(" Zoom") {
                    if let TheValue::Float(zoom) = value {
                        self.zoom = *zoom;
                        self.update_tiles(project, ui, ctx);
                    }
                }
            }
            _ => {}
        }
        redraw
    }

    ///  Create an id.
    fn make_id(&self, id: &str) -> String {
        self.id.to_owned() + id
    }
}
