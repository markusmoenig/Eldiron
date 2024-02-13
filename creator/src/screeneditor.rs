use shared::prelude::*;

use crate::editor::{CODEEDITOR, TILEDRAWER};
use crate::prelude::*;

#[derive(PartialEq, Clone, Copy, Debug)]
enum EditorMode {
    _Create,
}

pub struct ScreenEditor {
    // editor_mode: EditorMode,
}

#[allow(clippy::new_without_default)]
impl ScreenEditor {
    pub fn new() -> Self {
        Self {
            //editor_mode: EditorMode::Create,
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

        // Details

        let mut details_canvas = TheCanvas::new();

        let mut vlayout = TheVLayout::new(TheId::named("Widget Details Layout"));
        vlayout.set_margin(vec4i(5, 10, 5, 20));
        vlayout.set_alignment(TheHorizontalAlign::Left);
        vlayout.limiter_mut().set_max_width(150);

        let mut name_edit = TheTextLineEdit::new(TheId::named("Widget Name Edit"));
        name_edit.limiter_mut().set_max_width(130);
        name_edit.set_disabled(true);

        let mut text = TheText::new(TheId::empty());
        text.set_text_size(12.0);
        text.set_text("Widget Name".to_string());
        vlayout.add_widget(Box::new(text));
        vlayout.add_widget(Box::new(name_edit));

        let mut x_edit = TheTextLineEdit::new(TheId::named("Widget X Edit"));
        x_edit.limiter_mut().set_max_width(130);
        x_edit.set_disabled(true);

        let mut text = TheText::new(TheId::empty());
        text.set_text_size(12.0);
        text.set_text("Widget X".to_string());
        vlayout.add_widget(Box::new(text));
        vlayout.add_widget(Box::new(x_edit));

        let mut y_edit = TheTextLineEdit::new(TheId::named("Widget Y Edit"));
        y_edit.limiter_mut().set_max_width(130);
        y_edit.set_disabled(true);

        let mut text = TheText::new(TheId::empty());
        text.set_text_size(12.0);
        text.set_text("Widget Y".to_string());
        vlayout.add_widget(Box::new(text));
        vlayout.add_widget(Box::new(y_edit));

        let mut width_edit = TheTextLineEdit::new(TheId::named("Widget Width Edit"));
        width_edit.limiter_mut().set_max_width(130);
        width_edit.set_disabled(true);

        let mut text = TheText::new(TheId::empty());
        text.set_text_size(12.0);
        text.set_text("Widget Width".to_string());
        vlayout.add_widget(Box::new(text));
        vlayout.add_widget(Box::new(width_edit));

        let mut height_edit = TheTextLineEdit::new(TheId::named("Widget Height Edit"));
        height_edit.limiter_mut().set_max_width(130);
        height_edit.set_disabled(true);

        let mut text = TheText::new(TheId::empty());
        text.set_text_size(12.0);
        text.set_text("Widget Height".to_string());
        vlayout.add_widget(Box::new(text));
        vlayout.add_widget(Box::new(height_edit));

        let mut text = TheText::new(TheId::named("Sceeen Hover Position"));
        text.set_text_size(13.0);
        text.set_text("".to_string());
        vlayout.add_widget(Box::new(text));
        vlayout.set_reverse_index(Some(1));

        details_canvas.set_layout(vlayout);
        center.set_left(details_canvas);

        center
    }

    #[allow(clippy::too_many_arguments)]
    pub fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        client: &mut Client,
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
                                x: min_x as f32,
                                y: min_y as f32,
                                width: width as f32,
                                height: height as f32,
                                ..Default::default()
                            };

                            widget.bundle.id = widget.id;

                            let init = TheCodeGrid {
                                name: "init".into(),
                                ..Default::default()
                            };

                            widget.bundle.insert_grid(init);

                            let mut draw = TheCodeGrid {
                                name: "draw".into(),
                                ..Default::default()
                            };

                            draw.insert_atom(
                                (0, 0),
                                TheCodeAtom::ExternalCall(
                                    "Fill".to_string(),
                                    "Fills the widget with the given color.".to_string(),
                                    vec![str!("Color")],
                                    vec![TheValue::ColorObject(TheColor::default(), 0.0)],
                                    None,
                                ),
                            );

                            draw.insert_atom(
                                (2, 0),
                                TheCodeAtom::Value(TheValue::ColorObject(TheColor::default(), 0.0)),
                            );

                            widget.bundle.insert_grid(draw);

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
                                client.update_screen(screen);
                                redraw = true;
                            }
                        }
                    }
                }
            }
            TheEvent::TileEditorHoverChanged(id, pos) => {
                if id.name == "Screen Editor View" {
                    if let Some(text) = ui.get_text("Sceeen Hover Position") {
                        text.set_text(format!("({}, {})", pos.x, pos.y));
                        redraw = true;
                        ctx.ui.relayout = true;
                    }
                }
            }
            TheEvent::ValueChanged(id, value) => {
                if id.name == "Widget Name Edit" {
                    if let Some(screen) = project.screens.get_mut(&server_ctx.curr_screen) {
                        if let Some(widget_id) = server_ctx.curr_widget {
                            if let Some(widget) = screen.widgets.get_mut(&widget_id) {
                                widget.name = value.describe();
                                if let Some(list_item) = ui.get_widget_id(widget_id) {
                                    list_item.set_value(value.clone());
                                }
                            }
                        }
                    }
                } else if id.name == "Widget X Edit" {
                    if let Some(screen) = project.screens.get_mut(&server_ctx.curr_screen) {
                        if let Some(widget_id) = server_ctx.curr_widget {
                            if let Some(widget) = screen.widgets.get_mut(&widget_id) {
                                if let Some(v) = value.as_f32() {
                                    widget.x = v;
                                    client.update_screen(screen);
                                }
                            }
                        }
                    }
                } else if id.name == "Widget Y Edit" {
                    if let Some(screen) = project.screens.get_mut(&server_ctx.curr_screen) {
                        if let Some(widget_id) = server_ctx.curr_widget {
                            if let Some(widget) = screen.widgets.get_mut(&widget_id) {
                                if let Some(v) = value.as_f32() {
                                    widget.y = v;
                                    client.update_screen(screen);
                                }
                            }
                        }
                    }
                } else if id.name == "Widget Width Edit" {
                    if let Some(screen) = project.screens.get_mut(&server_ctx.curr_screen) {
                        if let Some(widget_id) = server_ctx.curr_widget {
                            if let Some(widget) = screen.widgets.get_mut(&widget_id) {
                                if let Some(v) = value.as_f32() {
                                    widget.width = v;
                                    client.update_screen(screen);
                                }
                            }
                        }
                    }
                } else if id.name == "Widget Height Edit" {
                    if let Some(screen) = project.screens.get_mut(&server_ctx.curr_screen) {
                        if let Some(widget_id) = server_ctx.curr_widget {
                            if let Some(widget) = screen.widgets.get_mut(&widget_id) {
                                if let Some(v) = value.as_f32() {
                                    widget.height = v;
                                    client.update_screen(screen);
                                }
                            }
                        }
                    }
                } else if id.name == "Screen Editor Zoom" {
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
                } else if id.name == "Screen Content List Item" {
                    if let Some(screen) = project.screens.get_mut(&server_ctx.curr_screen) {
                        if let Some(widget) = screen.widgets.get_mut(&id.uuid) {
                            let widget_list_canvas: TheCanvas = CODEEDITOR
                                .lock()
                                .unwrap()
                                .set_bundle(widget.bundle.clone(), ctx, 380, Some(200));

                            if let Some(stack_layout) = ui.get_stack_layout("List Stack Layout") {
                                if let Some(canvas) =
                                    stack_layout.canvas_at_mut(SidebarMode::Screen as usize)
                                {
                                    canvas.set_bottom(widget_list_canvas);
                                }
                            }
                            server_ctx.curr_widget = Some(widget.id);
                            self.apply_widget(ui, ctx, Some(widget));
                        }
                    }
                }
            }
            _ => {}
        }

        redraw
    }

    /// Redraw the map of the current screen on tick.
    pub fn redraw_screen(
        &mut self,
        ui: &mut TheUI,
        client: &mut Client,
        ctx: &mut TheContext,
        server_ctx: &ServerContext,
    ) {
        if let Some(rgba_layout) = ui.canvas.get_layout(Some(&"Screen Editor".into()), None) {
            if let Some(rgba_layout) = rgba_layout.as_rgba_layout() {
                if let Some(rgba_view) = rgba_layout.rgba_view_mut().as_rgba_view() {
                    if let Some(curr_character_instance) = server_ctx.curr_character_instance {
                        client.set_character_id(curr_character_instance);
                    }
                    client.draw_screen(
                        &server_ctx.curr_screen,
                        rgba_view.buffer_mut(),
                        &TILEDRAWER.lock().unwrap(),
                        ctx,
                        server_ctx,
                    );
                    rgba_view.set_needs_redraw(true);
                }
            }
        }
    }

    /// Apply the given widget to the UI
    pub fn apply_widget(&mut self, ui: &mut TheUI, ctx: &mut TheContext, widget: Option<&Widget>) {
        ui.set_widget_disabled_state("Widget Name Edit", ctx, widget.is_none());
        ui.set_widget_disabled_state("Widget X Edit", ctx, widget.is_none());
        ui.set_widget_disabled_state("Widget Y Edit", ctx, widget.is_none());
        ui.set_widget_disabled_state("Widget Width Edit", ctx, widget.is_none());
        ui.set_widget_disabled_state("Widget Height Edit", ctx, widget.is_none());

        if let Some(widget) = widget {
            ui.set_widget_value(
                "Widget Name Edit",
                ctx,
                TheValue::Text(widget.name.to_string()),
            );
            ui.set_widget_value("Widget X Edit", ctx, TheValue::Text(widget.x.to_string()));
            ui.set_widget_value("Widget Y Edit", ctx, TheValue::Text(widget.y.to_string()));
            ui.set_widget_value(
                "Widget Width Edit",
                ctx,
                TheValue::Text(widget.width.to_string()),
            );
            ui.set_widget_value(
                "Widget Height Edit",
                ctx,
                TheValue::Text(widget.height.to_string()),
            );
        } else {
            ui.set_widget_value("Widget Name Edit", ctx, TheValue::Text(str!("")));
            ui.set_widget_value("Widget X Edit", ctx, TheValue::Text(str!("")));
            ui.set_widget_value("Widget Y Edit", ctx, TheValue::Text(str!("")));
            ui.set_widget_value("Widget Width Edit", ctx, TheValue::Text(str!("")));
            ui.set_widget_value("Widget Height Edit", ctx, TheValue::Text(str!("")));
        }
    }
}
