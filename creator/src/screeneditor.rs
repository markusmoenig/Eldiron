use shared::prelude::*;

use crate::editor::CODEEDITOR;
use crate::prelude::*;

#[derive(PartialEq, Clone, Copy, Debug)]
enum ScreenEditorMode {
    Draw,
    Code,
    Pick,
    Erase,
}

#[derive(PartialEq, Clone, Copy, Debug)]
enum ScreenEditorDrawingMode {
    Background,
    Foreground,
}

pub struct ScreenEditor {
    editor_mode: ScreenEditorMode,
    drawing_mode: ScreenEditorDrawingMode,
    draw_outlines: bool,
    curr_tile_uuid: Option<Uuid>,
}

#[allow(clippy::new_without_default)]
impl ScreenEditor {
    pub fn new() -> Self {
        Self {
            editor_mode: ScreenEditorMode::Draw,
            drawing_mode: ScreenEditorDrawingMode::Background,
            draw_outlines: true,
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
            rgba_view.set_mode(TheRGBAViewMode::TileEditor);
            rgba_view.set_dont_show_grid(true);

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

        let mut max_text = TheText::new(TheId::named("Screen Grid Size Text"));
        //max_text.set_text_size(12.0);
        max_text.set_text(format!("Screen Grid: {0} x {1}", 0, 0));

        // let mut zoom = TheSlider::new(TheId::named("Screen Editor Zoom"));
        // zoom.set_value(TheValue::Float(1.0));
        // zoom.set_default_value(TheValue::Float(1.0));
        // zoom.set_range(TheValue::RangeF32(0.3..=3.0));
        // zoom.set_continuous(true);
        // zoom.limiter_mut().set_max_width(120);

        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(vec4i(10, 4, 5, 4));
        //toolbar_hlayout.add_widget(Box::new(gb));
        toolbar_hlayout.add_widget(Box::new(max_text));
        //toolbar_hlayout.add_widget(Box::new(zoom));
        toolbar_hlayout.set_reverse_index(Some(1));

        top_toolbar.set_layout(toolbar_hlayout);
        center.set_top(top_toolbar);

        // Bottom Toolbar
        let mut bottom_toolbar = TheCanvas::new();
        bottom_toolbar.set_widget(TheTraybar::new(TheId::empty()));

        let mut gb = TheGroupButton::new(TheId::named("Screen Editor Group"));
        gb.add_text_status_icon(
            "Draw".to_string(),
            "Draw a tile into the screen, either in the background or foreground.".to_string(),
            "draw".to_string(),
        );
        gb.add_text_status_icon(
            "Code".to_string(),
            "Code character and region behavior.".to_string(),
            "code".to_string(),
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
        gb.set_item_width(75);

        let mut layer_gb = TheGroupButton::new(TheId::named("Screen Editor Layer Group"));
        layer_gb.add_text_status(
            "Background".to_string(),
            "Drawing works on the background tiles (before widgets are drawn).".to_string(),
        );
        layer_gb.add_text_status(
            "Foreground".to_string(),
            "Drawing works on the foreground tiles (after widgets are drawn).".to_string(),
        );
        layer_gb.set_item_width(100);

        let mut drop_down = TheDropdownMenu::new(TheId::named("Widget Outlines"));
        drop_down.add_option("Show Outlines".to_string());
        drop_down.add_option("No Outlines".to_string());
        drop_down.set_status_text("Toggles the visibility of widget outlines.");

        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(vec4i(10, 4, 10, 4));
        toolbar_hlayout.add_widget(Box::new(gb));

        let mut spacer = TheSpacer::new(TheId::empty());
        spacer.limiter_mut().set_max_size(vec2i(10, 5));
        toolbar_hlayout.add_widget(Box::new(spacer));

        toolbar_hlayout.add_widget(Box::new(layer_gb));
        toolbar_hlayout.add_widget(Box::new(drop_down));
        toolbar_hlayout.set_reverse_index(Some(1));

        bottom_toolbar.set_layout(toolbar_hlayout);
        center.set_bottom(bottom_toolbar);

        // Details

        let mut details_canvas = TheCanvas::new();

        let mut vlayout = TheVLayout::new(TheId::named("Widget Details Layout"));
        vlayout.set_margin(vec4i(10, 10, 5, 10));
        vlayout.set_alignment(TheHorizontalAlign::Left);
        vlayout.limiter_mut().set_max_width(90);
        vlayout.set_background_color(Some(TheThemeColors::ListLayoutBackground));

        let mut x_edit = TheTextLineEdit::new(TheId::named("Widget X Edit"));
        x_edit.limiter_mut().set_max_width(70);
        x_edit.set_disabled(true);

        let mut text = TheText::new(TheId::empty());
        text.set_text_size(12.0);
        text.set_text("Widget X".to_string());
        vlayout.add_widget(Box::new(text));
        vlayout.add_widget(Box::new(x_edit));

        let mut y_edit = TheTextLineEdit::new(TheId::named("Widget Y Edit"));
        y_edit.limiter_mut().set_max_width(70);
        y_edit.set_disabled(true);

        let mut text = TheText::new(TheId::empty());
        text.set_text_size(12.0);
        text.set_text("Widget Y".to_string());
        vlayout.add_widget(Box::new(text));
        vlayout.add_widget(Box::new(y_edit));

        let mut width_edit = TheTextLineEdit::new(TheId::named("Widget Width Edit"));
        width_edit.limiter_mut().set_max_width(70);
        width_edit.set_disabled(true);

        let mut text = TheText::new(TheId::empty());
        text.set_text_size(12.0);
        text.set_text("Width".to_string());
        vlayout.add_widget(Box::new(text));
        vlayout.add_widget(Box::new(width_edit));

        let mut height_edit = TheTextLineEdit::new(TheId::named("Widget Height Edit"));
        height_edit.limiter_mut().set_max_width(70);
        height_edit.set_disabled(true);

        let mut text = TheText::new(TheId::empty());
        text.set_text_size(12.0);
        text.set_text("Height".to_string());
        vlayout.add_widget(Box::new(text));
        vlayout.add_widget(Box::new(height_edit));

        // let mut spacer = TheSpacer::new(TheId::empty());
        // spacer.limiter_mut().set_max_height(2);
        // vlayout.add_widget(Box::new(max_text));
        // vlayout.add_widget(Box::new(spacer));

        let mut text = TheText::new(TheId::named("Sceeen Hover Position"));
        text.set_text_size(13.0);
        text.set_text("".to_string());
        text.set_text_color([200, 200, 200, 255]);
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
            TheEvent::IndexChanged(id, index) => {
                if id.name == "Widget Outlines" {
                    self.draw_outlines = *index == 0;
                } else if id.name == "Screen Editor Layer Group" {
                    if *index == ScreenEditorDrawingMode::Background as usize {
                        self.drawing_mode = ScreenEditorDrawingMode::Background;
                    } else if *index == ScreenEditorDrawingMode::Foreground as usize {
                        self.drawing_mode = ScreenEditorDrawingMode::Foreground;
                    }
                } else if id.name == "Screen Editor Group" {
                    if let Some(rgba_layout) = ui.get_rgba_layout("Screen Editor") {
                        if let Some(rgba_view) = rgba_layout.rgba_view_mut().as_rgba_view() {
                            if *index == ScreenEditorMode::Draw as usize {
                                ctx.ui.send(TheEvent::Custom(
                                    TheId::named("Set Tilepicker Panel"),
                                    TheValue::Empty,
                                ));
                                self.editor_mode = ScreenEditorMode::Draw;
                            } else if *index == ScreenEditorMode::Code as usize {
                                ctx.ui.send(TheEvent::Custom(
                                    TheId::named("Set CodeGrid Panel"),
                                    TheValue::Empty,
                                ));
                                rgba_view.set_mode(TheRGBAViewMode::TileEditor);
                            } else if *index == ScreenEditorMode::Pick as usize {
                                self.editor_mode = ScreenEditorMode::Pick;
                            } else if *index == ScreenEditorMode::Erase as usize {
                                self.editor_mode = ScreenEditorMode::Erase;
                            }
                        }
                    }
                }
            }
            TheEvent::TileEditorClicked(_id, coord) => {
                if self.editor_mode == ScreenEditorMode::Draw {
                    if let Some(screen) = project.screens.get_mut(&server_ctx.curr_screen) {
                        if let Some(tile_id) = self.curr_tile_uuid {
                            if self.drawing_mode == ScreenEditorDrawingMode::Background {
                                screen.add_background_tile((coord.x, coord.y), tile_id);
                            } else if self.drawing_mode == ScreenEditorDrawingMode::Foreground {
                                screen.add_foreground_tile((coord.x, coord.y), tile_id);
                            }
                            client.update_screen(screen);
                            redraw = true;
                        }
                    }
                } else if self.editor_mode == ScreenEditorMode::Pick
                    || self.editor_mode == ScreenEditorMode::Erase
                {
                    if let Some(screen) = project.screens.get_mut(&server_ctx.curr_screen) {
                        if self.editor_mode == ScreenEditorMode::Erase {
                            if self.drawing_mode == ScreenEditorDrawingMode::Background {
                                screen.erase_background_tile((coord.x, coord.y));
                            } else if self.drawing_mode == ScreenEditorDrawingMode::Foreground {
                                screen.erase_foreground_tile((coord.x, coord.y));
                            }
                            client.update_screen(screen);
                            redraw = true;
                        }

                        let sorted_widgets = screen.sorted_widgets_by_size();
                        for widget in sorted_widgets.iter() {
                            if widget.is_inside(coord) && self.editor_mode == ScreenEditorMode::Pick
                            {
                                if let Some(layout) = ui.get_list_layout("Screen Content List") {
                                    layout.select_item(widget.id, ctx, true);
                                }
                                /*else if self.editor_mode == ScreenEditorMode::Erase {
                                open_delete_confirmation_dialog(
                                    "Delete Widget ?",
                                    format!("Permanently delete '{}' ?", widget.name).as_str(),
                                    widget.id,
                                    ui,
                                    ctx,
                                    );
                                    }*/
                            }
                        }
                    }
                }

                // Handle actual game interaction
                if self.editor_mode == ScreenEditorMode::Pick {
                    if let Some(screen) = project.screens.get_mut(&server_ctx.curr_screen) {
                        client.touch_down(
                            &server_ctx.curr_screen,
                            vec2i(coord.x * screen.grid_size, coord.y * screen.grid_size),
                        );
                    }
                }
            }
            TheEvent::TileEditorUp(_id) => {
                if self.editor_mode == ScreenEditorMode::Pick {
                    client.touch_up(&server_ctx.curr_screen);
                }
            }
            /*
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
                                    vec![TheValue::ColorObject(TheColor::default())],
                                    None,
                                ),
                            );

                            draw.insert_atom(
                                (2, 0),
                                TheCodeAtom::Value(TheValue::ColorObject(TheColor::default())),
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
                                screen.widget_list.push(widget);
                                client.update_screen(screen);
                                redraw = true;
                            }
                        }
                    }
                }
                }*/
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
                if id.name == "Widget X Edit" {
                    if let Some(screen) = project.screens.get_mut(&server_ctx.curr_screen) {
                        if let Some(widget_id) = server_ctx.curr_widget {
                            if let Some(widget) = screen.get_widget_mut(&widget_id) {
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
                            if let Some(widget) = screen.get_widget_mut(&widget_id) {
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
                            if let Some(widget) = screen.get_widget_mut(&widget_id) {
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
                            if let Some(widget) = screen.get_widget_mut(&widget_id) {
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
                if id.name == "Widget Move Up" {
                    if let Some(screen) = project.screens.get_mut(&server_ctx.curr_screen) {
                        if let Some(widget_id) = server_ctx.curr_widget {
                            let mut index = 0;
                            for (i, w) in screen.widget_list.iter().enumerate() {
                                if w.id == widget_id {
                                    index = i;
                                    break;
                                }
                            }
                            if index > 0 {
                                screen.widget_list.swap(index, index - 1);
                                client.update_screen(screen);
                                ui.select_first_list_item("Screen List", ctx);
                                // ui.select_list_item_at("Screen List", index as i32 - 1, ctx);
                            }
                        }
                    }
                } else if id.name == "Widget Move Down" {
                    if let Some(screen) = project.screens.get_mut(&server_ctx.curr_screen) {
                        if let Some(widget_id) = server_ctx.curr_widget {
                            let mut index = 0;
                            for (i, w) in screen.widget_list.iter().enumerate() {
                                if w.id == widget_id {
                                    index = i;
                                    break;
                                }
                            }
                            if index < screen.widget_list.len() - 1 {
                                screen.widget_list.swap(index, index + 1);
                                client.update_screen(screen);
                                ui.select_first_list_item("Screen List", ctx);
                                //ui.select_list_item("Screen List", &widget_id, ctx);
                            }
                        }
                    }
                } else if id.name == "Screen Item" {
                    if let Some(screen) = project.screens.get_mut(&server_ctx.curr_screen) {
                        if let Some(rgba_layout) =
                            ui.canvas.get_layout(Some(&"Screen Editor".into()), None)
                        {
                            if let Some(rgba_layout) = rgba_layout.as_rgba_layout() {
                                if let Some(rgba_view) = rgba_layout.rgba_view_mut().as_rgba_view()
                                {
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
                        ui.set_widget_value(
                            "Screen Grid Size Text",
                            ctx,
                            TheValue::Text(format!(
                                "Screen Grid: {0} x {1}",
                                screen.width / screen.grid_size,
                                screen.height / screen.grid_size
                            )),
                        );

                        if screen.bundle.grids.is_empty() {
                            let init = TheCodeGrid {
                                name: "init".into(),
                                ..Default::default()
                            };
                            screen.bundle.insert_grid(init);

                            let draw = TheCodeGrid {
                                name: "draw".into(),
                                ..Default::default()
                            };
                            screen.bundle.insert_grid(draw);
                        }

                        let screen_list_canvas: TheCanvas = CODEEDITOR.lock().unwrap().set_bundle(
                            screen.bundle.clone(),
                            ctx,
                            380,
                            Some(200),
                        );

                        if let Some(stack_layout) = ui.get_stack_layout("List Stack Layout") {
                            if let Some(canvas) =
                                stack_layout.canvas_at_mut(SidebarMode::Screen as usize)
                            {
                                canvas.set_bottom(screen_list_canvas);
                            }
                        }
                        //self.redraw_region(ui, server, ctx, server_ctx);
                        redraw = true;
                    }
                } else if id.name == "Screen Content List Item" {
                    if let Some(screen) = project.screens.get_mut(&server_ctx.curr_screen) {
                        if let Some(widget) = screen.get_widget_mut(&id.uuid) {
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
                } else if id.name == "Tilemap Tile" {
                    self.curr_tile_uuid = Some(id.uuid);
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
        _ctx: &mut TheContext,
        server_ctx: &ServerContext,
        project: &Project,
    ) {
        if let Some(rgba_layout) = ui.canvas.get_layout(Some(&"Screen Editor".into()), None) {
            if let Some(rgba_layout) = rgba_layout.as_rgba_layout() {
                if let Some(rgba_view) = rgba_layout.rgba_view_mut().as_rgba_view() {
                    if let Some(curr_character_instance) = server_ctx.curr_character_instance {
                        client.set_character_id(curr_character_instance);
                    }

                    client.draw_screen(&server_ctx.curr_screen, rgba_view.buffer_mut());
                    rgba_view.set_needs_redraw(true);

                    if self.draw_outlines {
                        if let Some(screen) = project.screens.get(&server_ctx.curr_screen) {
                            let gs = screen.grid_size;
                            for widget in &screen.widget_list {
                                let x = (widget.x * gs as f32).max(0.0) as i32;
                                let y = (widget.y * gs as f32).max(0.0) as i32;
                                let width = widget.width as i32;
                                let height = widget.height as i32;

                                if Some(widget.id) != server_ctx.curr_widget {
                                    rgba_view.buffer_mut().draw_rect_outline(
                                        &TheDim::new(x, y, width * gs, height * gs),
                                        &[128, 128, 128, 255],
                                    );
                                }
                            }
                            for widget in &screen.widget_list {
                                let x = (widget.x * gs as f32).max(0.0) as i32;
                                let y = (widget.y * gs as f32).max(0.0) as i32;
                                let width = widget.width as i32;
                                let height = widget.height as i32;

                                if Some(widget.id) == server_ctx.curr_widget {
                                    rgba_view.buffer_mut().draw_rect_outline(
                                        &TheDim::new(x, y, width * gs, height * gs),
                                        &[255, 255, 255, 255],
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Apply the given widget to the UI
    pub fn apply_widget(&mut self, ui: &mut TheUI, ctx: &mut TheContext, widget: Option<&Widget>) {
        ui.set_widget_disabled_state("Widget X Edit", ctx, widget.is_none());
        ui.set_widget_disabled_state("Widget Y Edit", ctx, widget.is_none());
        ui.set_widget_disabled_state("Widget Width Edit", ctx, widget.is_none());
        ui.set_widget_disabled_state("Widget Height Edit", ctx, widget.is_none());

        if let Some(widget) = widget {
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
            ui.set_widget_value("Widget X Edit", ctx, TheValue::Text(str!("")));
            ui.set_widget_value("Widget Y Edit", ctx, TheValue::Text(str!("")));
            ui.set_widget_value("Widget Width Edit", ctx, TheValue::Text(str!("")));
            ui.set_widget_value("Widget Height Edit", ctx, TheValue::Text(str!("")));
        }
    }
}
