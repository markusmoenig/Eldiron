//use crate::editor::{CODEEDITOR, SIDEBARMODE, TILEDRAWER};
use crate::prelude::*;

#[derive(PartialEq, Clone, Copy, Debug)]
enum EditorMode {
    Create,
    Pick,
    Erase,
}

pub struct ScreenEditor {
    editor_mode: EditorMode,

    curr_tile_uuid: Option<Uuid>,
}

#[allow(clippy::new_without_default)]
impl ScreenEditor {
    pub fn new() -> Self {
        Self {
            editor_mode: EditorMode::Create,

            curr_tile_uuid: None,
        }
    }

    pub fn init_ui(
        &mut self,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        _project: &mut Project,
    ) -> TheCanvas {
        let mut center = TheCanvas::new();

        let mut screen_editor = TheRGBALayout::new(TheId::named("Screen Editor"));
        if let Some(rgba_view) = screen_editor.rgba_view_mut().as_rgba_view() {
            rgba_view.set_mode(TheRGBAViewMode::Display);

            if let Some(buffer) = ctx.ui.icon("eldiron_map") {
                rgba_view.set_buffer(buffer.clone());
            }

            rgba_view.set_grid_color([255, 255, 255, 5]);
            rgba_view.set_hover_color(Some([255, 255, 255, 100]));
            rgba_view.set_wheel_scale(-0.2);
        }

        center.set_layout(screen_editor);

        // Top Toolbar
        let mut top_toolbar = TheCanvas::new();
        top_toolbar.set_widget(TheTraybar::new(TheId::empty()));

        // let mut gb = TheGroupButton::new(TheId::named("2D3D Group"));
        // gb.add_text("2D Map".to_string());
        // gb.add_text("Mixed".to_string());
        // gb.add_text("3D Map".to_string());

        let mut zoom = TheSlider::new(TheId::named("Screen Editor Zoom"));
        zoom.set_value(TheValue::Float(1.0));
        zoom.set_range(TheValue::RangeF32(0.5..=3.0));
        zoom.set_continuous(true);
        zoom.limiter_mut().set_max_width(120);

        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(vec4i(5, 4, 5, 4));
        //toolbar_hlayout.add_widget(Box::new(gb));
        toolbar_hlayout.add_widget(Box::new(zoom));
        toolbar_hlayout.set_reverse_index(Some(1));

        top_toolbar.set_layout(toolbar_hlayout);
        center.set_top(top_toolbar);

        // Bottom Toolbar
        let mut bottom_toolbar = TheCanvas::new();
        bottom_toolbar.set_widget(TheTraybar::new(TheId::empty()));

        let mut gb = TheGroupButton::new(TheId::named("Editor Group"));
        gb.add_text_status_icon(
            "Create".to_string(),
            "Create a widget via rectangular selection.".to_string(),
            "draw".to_string(),
        );
        gb.add_text_status_icon(
            "Pick".to_string(),
            "Pick a widget in the screen.".to_string(),
            "pick".to_string(),
        );
        gb.add_text_status_icon(
            "Erase".to_string(),
            "Delete a widget in the screen.".to_string(),
            "eraser".to_string(),
        );
        gb.set_item_width(65);

        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(vec4i(5, 4, 5, 4));
        toolbar_hlayout.add_widget(Box::new(gb));

        bottom_toolbar.set_layout(toolbar_hlayout);
        center.set_bottom(bottom_toolbar);

        center
    }

    pub fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        _server: &mut Server,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let mut redraw = false;
        match event {
            TheEvent::TileSelectionChanged(id) => {
                if id.name == "Screen Editor View" {
                    if let Some(rgba_layout) = ui.get_rgba_layout("Screen Editor") {
                        if let Some(rgba_view) = rgba_layout.rgba_view_mut().as_rgba_view() {
                            let selection = rgba_view.selection();
                            rgba_view.set_selection(FxHashSet::default());

                            let mut min_x = i32::MAX;
                            let mut max_x = i32::MIN;
                            let mut min_y = i32::MAX;
                            let mut max_y = i32::MIN;

                            for &(x, y) in selection.iter() {
                                if x < min_x {
                                    min_x = x;
                                }
                                if x > max_x {
                                    max_x = x;
                                }
                                if y < min_y {
                                    min_y = y;
                                }
                                if y > max_y {
                                    max_y = y;
                                }
                            }

                            let width = max_x - min_x + 1;
                            let height = max_y - min_y + 1;

                            let mut widget = Widget {
                                x: min_x,
                                y: min_y,
                                width,
                                height,
                                ..Default::default()
                            };

                            let init = TheCodeGrid {
                                name: "init".into(),
                                ..Default::default()
                            };

                            widget.bundle.insert_grid(init);

                            let main = TheCodeGrid {
                                name: "draw".into(),
                                ..Default::default()
                            };
                            widget.bundle.insert_grid(main);

                            if let Some(screen) = project.screens.get_mut(&server_ctx.curr_screen) {
                                if let Some(list) = ui.get_list_layout("Screen Content List") {
                                    let mut list_item = TheListItem::new(TheId::named_with_id(
                                        "Screen Content List Item",
                                        widget.id,
                                    ));
                                    list_item.set_text(widget.name.clone());
                                    list_item.set_state(TheWidgetState::Selected);
                                    list_item.add_value_column(
                                        100,
                                        TheValue::Text("Widget".to_string()),
                                    );

                                    list.deselect_all();
                                    list.add_item(list_item, ctx);
                                    list.select_item(widget.id, ctx, true);
                                }
                                screen.widgets.insert(widget.id, widget);
                                redraw = true;
                            }
                        }
                    }
                }
            }
            TheEvent::StateChanged(id, _state) => {
                if id.name == "Screen Item" {
                    if let Some(screen) = project.screens.get_mut(&server_ctx.curr_screen) {
                        if let Some(rgba_layout) =
                            ui.canvas.get_layout(Some(&"Screen Editor".into()), None)
                        {
                            if let Some(rgba_layout) = rgba_layout.as_rgba_layout() {
                                if let Some(rgba_view) = rgba_layout.rgba_view_mut().as_rgba_view()
                                {
                                    rgba_view.set_mode(TheRGBAViewMode::TileSelection);
                                    rgba_view.set_rectangular_selection(true);
                                    let width = screen.width;
                                    let height = screen.height;
                                    let buffer =
                                        TheRGBABuffer::new(TheDim::new(0, 0, width, height));
                                    rgba_view.set_buffer(buffer);
                                    rgba_view.set_grid(Some(screen.grid_size));
                                    ctx.ui.relayout = true;
                                }
                                rgba_layout.scroll_to(screen.scroll_offset);
                            }
                        }
                        server_ctx.curr_screen = screen.id;
                        //self.redraw_region(ui, server, ctx, server_ctx);
                        redraw = true;
                    }
                }
            }
            TheEvent::ValueChanged(id, value) => {
                if id.name == "Screen Editor Zoom" {
                    if let Some(v) = value.to_f32() {
                        if let Some(screen) = project.screens.get_mut(&server_ctx.curr_screen) {
                            screen.zoom = v;
                        }
                        if let Some(layout) = ui.get_rgba_layout("Screen Editor") {
                            layout.set_zoom(v);
                            layout.relayout(ctx);
                        }
                    }
                }
            }
            _ => {}
        }

        redraw
    }
}
