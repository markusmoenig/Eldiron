use crate::editor::{CODEEDITOR, TILEDRAWER, TILEMAPEDITOR, TILEPICKER};
use crate::prelude::*;

pub struct Panels {
    pub curr_atom: Option<TheCodeAtom>,
}

#[allow(clippy::new_without_default)]
impl Panels {
    pub fn new() -> Self {
        let mut codeeditor = CODEEDITOR.lock().unwrap();

        codeeditor.add_external(TheExternalCode::new(
            "KeyDown".to_string(),
            "Returns the currently pressed key (if any).".to_string(),
            vec![],
            vec![],
            Some(TheValue::Text("".to_string())),
        ));

        /*
        codeeditor.add_external(TheExternalCode::new(
            "RandWalk".to_string(),
            "Moves the character in a random direction.".to_string(),
            vec![],
            vec![],
            None,
        ));*/

        codeeditor.add_external(TheExternalCode::new(
            "Pulse".to_string(),
            "Counts up to the value in \"Count to\" and returns true on completion. Then restarts."
                .to_string(),
            vec!["Count to".to_string()],
            vec![TheValue::Int(4)],
            Some(TheValue::Bool(false)),
        ));

        codeeditor.add_external(TheExternalCode::new(
            "Move".to_string(),
            "Moves the character in the specified direction.".to_string(),
            vec!["By".to_string()],
            vec![TheValue::Float2(vec2f(0.0, 0.0))],
            Some(TheValue::Bool(false)),
        ));

        codeeditor.add_external(TheExternalCode::new(
            "InArea".to_string(),
            "Returns the amount of characters in the area.".to_string(),
            vec![],
            vec![],
            Some(TheValue::Int(0)),
        ));

        codeeditor.add_external(TheExternalCode::new(
            "Create".to_string(),
            "Creates the item identified by its name.".to_string(),
            vec![str!("Item")],
            vec![TheValue::Text(str!("name"))],
            Some(TheValue::CodeObject(TheCodeObject::default())),
        ));

        codeeditor.add_external(TheExternalCode::new(
            "WallFX".to_string(),
            "Applies an effect on the wall at the given position.".to_string(),
            vec!["Position".to_string(), "FX".to_string()],
            vec![
                TheValue::Position(vec3f(0.0, 0.0, 0.0)),
                TheValue::TextList(
                    0,
                    vec![
                        "Normal".to_string(),
                        "Move Up".to_string(),
                        "Move Right".to_string(),
                        "Move Down".to_string(),
                        "Move Left".to_string(),
                        "Fade Out".to_string(),
                    ],
                ),
            ],
            None,
        ));

        codeeditor.add_external(TheExternalCode::new(
            "Debug".to_string(),
            "Outputs the specified debug value.".to_string(),
            vec!["Value".to_string()],
            vec![TheValue::Text("Text".to_string())],
            None,
        ));

        Self { curr_atom: None }
    }

    pub fn init_ui(&mut self, ui: &mut TheUI, ctx: &mut TheContext, _project: &mut Project) {
        let mut canvas = TheCanvas::new();

        //let mut tab_layout = TheTabLayout::new(TheId::named("Browser"));
        //tab_layout.limiter_mut().set_max_height(300);

        let mut shared_layout = TheSharedHLayout::new(TheId::named("Shared Panel Layout"));
        shared_layout.limiter_mut().set_max_height(275);
        shared_layout.set_shared_ratio(0.75);
        //shared_layout.set_mode(TheSharedLayoutMode::Shared);

        // Left Stack

        let mut left_canvas = TheCanvas::new();
        let mut left_stack = TheStackLayout::new(TheId::named("Left Stack"));

        left_stack.add_canvas(TILEPICKER.lock().unwrap().build(false));
        left_stack.add_canvas(CODEEDITOR.lock().unwrap().build_canvas(ctx));
        left_stack.add_canvas(TILEMAPEDITOR.lock().unwrap().build());

        left_stack.set_index(0);

        let tilemap_editor = TheRGBALayout::new(TheId::named("Tilemap Editor"));
        let mut tilemap_canvas = TheCanvas::new();
        tilemap_canvas.set_layout(tilemap_editor);
        left_stack.add_canvas(tilemap_canvas);

        left_canvas.set_layout(left_stack);

        // Right Stack

        let mut right_canvas = TheCanvas::new();
        let mut right_stack = TheStackLayout::new(TheId::named("Right Stack"));

        // Context Group

        let mut context_group: TheGroupButton =
            TheGroupButton::new(TheId::named("Right Stack Group"));
        context_group.add_text_status(
            "Context".to_string(),
            "Shows the visual context of the selected code.".to_string(),
        );
        context_group.add_text_status(
            "Object".to_string(),
            "Shows the object properties of the current character or area.".to_string(),
        );
        context_group.add_text_status("Output".to_string(), "Shows the text output for the current character. Only available when the server is running.".to_string());

        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(vec4i(10, 2, 5, 2));

        // let mut text = TheText::new(TheId::named("Panel Object Text"));
        // text.set_text("Object".to_string());
        toolbar_hlayout.add_widget(Box::new(context_group));

        let mut toolbar_canvas = TheCanvas::default();
        toolbar_canvas.set_widget(TheTraybar::new(TheId::empty()));
        toolbar_canvas.set_layout(toolbar_hlayout);
        right_canvas.set_top(toolbar_canvas);

        // Context

        let mut codecontext_canvas = TheCanvas::new();
        let codecontext_layout = TheListLayout::new(TheId::named("CodeObject Context Layout"));
        codecontext_canvas.set_layout(codecontext_layout);

        right_stack.add_canvas(codecontext_canvas);

        // Object

        let mut codeobject_canvas = TheCanvas::new();
        let codeobject_layout = TheListLayout::new(TheId::named("CodeObject Layout"));
        codeobject_canvas.set_layout(codeobject_layout);

        right_stack.add_canvas(codeobject_canvas);

        // Out

        let mut out_canvas = TheCanvas::new();

        let codeobject_layout = TheListLayout::new(TheId::named("CodeObject Output Layout"));
        out_canvas.set_layout(codeobject_layout);

        right_stack.add_canvas(out_canvas);

        //

        right_canvas.set_layout(right_stack);

        //

        shared_layout.add_canvas(left_canvas);
        shared_layout.add_canvas(right_canvas);

        let mut status_canvas = TheCanvas::new();
        let mut statusbar = TheStatusbar::new(TheId::named("Statusbar"));
        statusbar.set_text(
            "Welcome to Eldiron! Visit Eldiron.com for information and example projects."
                .to_string(),
        );
        status_canvas.set_widget(statusbar);

        canvas.set_bottom(status_canvas);
        canvas.set_layout(shared_layout);

        ui.canvas.set_bottom(canvas);
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
        let mut redraw = CODEEDITOR.lock().unwrap().handle_event(event, ui, ctx);
        if TILEPICKER
            .lock()
            .unwrap()
            .handle_event(event, ui, ctx, project)
        {
            redraw = true;
        }

        match event {
            TheEvent::CodeEditorSelectionChanged(_, _) | TheEvent::CodeBundleChanged(_, _) => {
                let mut set_to = TheCanvas::new();
                let mut set_already = false;

                if let Some(atom) = CODEEDITOR.lock().unwrap().get_selected_atom(ui) {
                    //println!("Selected Atom: {:?}", atom);
                    self.curr_atom = Some(atom.clone());
                    if let TheCodeAtom::Value(TheValue::Position(pos)) = &atom {
                        let mut w = TheIconView::new(TheId::empty());
                        if let Some(tile) = project.extract_region_tile(
                            server_ctx.curr_region,
                            (pos.x as i32, pos.z as i32),
                        ) {
                            w.set_rgba_tile(tile);
                        }
                        set_to.set_widget(w);
                        set_already = true;
                    }
                    if let TheCodeAtom::Value(TheValue::Tile(name, _id)) = &atom {
                        let mut w = TheIconView::new(TheId::empty());
                        let tiledrawer = TILEDRAWER.lock().unwrap();
                        if let Some(found_id) = tiledrawer.get_tile_id_by_name(name.clone()) {
                            if let Some(tile) = tiledrawer.tiles.get(&found_id) {
                                w.set_rgba_tile(tile.clone());
                            }
                        }
                        set_to.set_widget(w);
                        set_already = true;
                    }
                    if let TheCodeAtom::Value(TheValue::ColorObject(color, _)) = &atom {
                        let mut vlayout = TheVLayout::new(TheId::empty());

                        let mut w = TheColorPicker::new(TheId::named("Atom Color Picker"));
                        w.set_value(TheValue::ColorObject(color.clone(), 0.0));
                        vlayout.set_background_color(Some(ListLayoutBackground));
                        vlayout.set_margin(vec4i(20, 20, 20, 20));
                        vlayout.add_widget(Box::new(w));
                        set_to.set_layout(vlayout);
                        set_already = true;
                    }
                    if let TheCodeAtom::Value(TheValue::Direction(value, randomness)) = &atom {
                        let mut vlayout = TheVLayout::new(TheId::empty());

                        let mut w = TheDirectionPicker::new(TheId::named("Atom Direction Picker"));
                        w.set_value(TheValue::Direction(*value, *randomness));
                        vlayout.set_background_color(Some(ListLayoutBackground));
                        vlayout.set_margin(vec4i(20, 20, 20, 20));
                        vlayout.add_widget(Box::new(w));
                        set_to.set_layout(vlayout);
                        set_already = true;
                    }
                } else {
                    self.curr_atom = None;
                }

                if !set_already {
                    let layout = TheListLayout::new(TheId::named("CodeObject Context Layout"));
                    set_to.set_layout(layout);
                } else {
                    ctx.ui
                        .send(TheEvent::SetStackIndex(TheId::named("Right Stack"), 0));
                    ui.set_widget_value("Right Stack Group", ctx, TheValue::Int(0));
                }

                if let Some(stack) = ui.get_stack_layout("Right Stack") {
                    if let Some(replace) = stack.canvas_at_mut(0) {
                        *replace = set_to;
                        ctx.ui.relayout = true;
                    }
                }
            }
            TheEvent::ValueChanged(id, value) => {
                if id.name == "Atom Color Picker" || id.name == "Atom Direction Picker" {
                    let mut editor = CODEEDITOR.lock().unwrap();
                    editor.start_undo(ui);
                    editor.set_selected_atom(ui, TheCodeAtom::Value(value.clone()));
                    editor.finish_undo(ui, ctx);
                    editor.set_grid_selection_ui(ui, ctx);
                    redraw = true;
                }
            }
            TheEvent::IndexChanged(id, index) => {
                if id.name == "Right Stack Group" {
                    if let Some(stack) = ui.get_stack_layout("Right Stack") {
                        stack.set_index(*index);
                        redraw = true;
                        ctx.ui.relayout = true;
                    }
                }
            }
            TheEvent::Custom(id, _) => {
                if id.name == "Set Region Panel" {
                    let mut shared_left = true;

                    if let Some(character) = server_ctx.curr_character_instance {
                        // Character
                        ctx.ui
                            .send(TheEvent::SetStackIndex(TheId::named("Right Stack"), 1));
                        ui.set_widget_value("Right Stack Group", ctx, TheValue::Int(1));

                        shared_left = false;

                        // If in Pick mode show the instance
                        if self.get_editor_group_index(ui) == 1 {
                            ctx.ui
                                .send(TheEvent::SetStackIndex(TheId::named("Left Stack"), 1));

                            if let Some(layout) = ui.get_sharedhlayout("Shared Panel Layout") {
                                layout.set_mode(TheSharedHLayoutMode::Shared);
                                ctx.ui.relayout = true;
                                redraw = true;
                            }

                            ui.set_widget_value("Right Stack Group", ctx, TheValue::Int(1));

                            if let Some((name, _)) = server.get_character_property(
                                server_ctx.curr_region,
                                character,
                                "name".into(),
                            ) {
                                if let Some(text) = ui.get_text("Panel Object Text") {
                                    text.set_text(name.describe());
                                }
                            }

                            self.update_code_object(ui, ctx, server, server_ctx);
                        }
                    } else if let Some(item) = server_ctx.curr_item_instance {
                        // Item
                        ctx.ui
                            .send(TheEvent::SetStackIndex(TheId::named("Right Stack"), 1));
                        ui.set_widget_value("Right Stack Group", ctx, TheValue::Int(1));

                        shared_left = false;

                        // If in Pick mode show the instance
                        if self.get_editor_group_index(ui) == 1 {
                            ctx.ui
                                .send(TheEvent::SetStackIndex(TheId::named("Left Stack"), 1));

                            if let Some(layout) = ui.get_sharedhlayout("Shared Panel Layout") {
                                layout.set_mode(TheSharedHLayoutMode::Shared);
                                ctx.ui.relayout = true;
                                redraw = true;
                            }

                            ui.set_widget_value("Right Stack Group", ctx, TheValue::Int(1));

                            if let Some((name, _)) = server.get_item_property(
                                server_ctx.curr_region,
                                item,
                                "name".into(),
                            ) {
                                if let Some(text) = ui.get_text("Panel Object Text") {
                                    text.set_text(name.describe());
                                }
                            }

                            self.update_code_object(ui, ctx, server, server_ctx);
                        }
                    } else if let Some(area_id) = server_ctx.curr_area {
                        // Area
                        ctx.ui
                            .send(TheEvent::SetStackIndex(TheId::named("Right Stack"), 1));

                        // If in Pick mode show the instance
                        if self.get_editor_group_index(ui) == 1 {
                            ctx.ui
                                .send(TheEvent::SetStackIndex(TheId::named("Left Stack"), 1));

                            if let Some(layout) = ui.get_sharedhlayout("Shared Panel Layout") {
                                layout.set_mode(TheSharedHLayoutMode::Shared);
                                ctx.ui.relayout = true;
                                redraw = true;
                                shared_left = false;
                            }

                            ui.set_widget_value("Right Stack Group", ctx, TheValue::Int(1));

                            if let Some(region) = project.get_region(&server_ctx.curr_region) {
                                if let Some(area) = region.areas.get(&area_id) {
                                    if let Some(text) = ui.get_text("Panel Object Text") {
                                        text.set_text(area.name.clone());
                                    }
                                }
                            }

                            self.update_code_object(ui, ctx, server, server_ctx);
                        }
                    } else {
                        ctx.ui
                            .send(TheEvent::SetStackIndex(TheId::named("Left Stack"), 0));
                    }

                    if shared_left {
                        if let Some(layout) = ui.get_sharedhlayout("Shared Panel Layout") {
                            layout.set_mode(TheSharedHLayoutMode::Left);
                            ctx.ui.relayout = true;
                            redraw = true;
                        }
                    }
                } else if id.name == "Set CodeGrid Panel" {
                    //println!("Set CodeGrid Panel");
                    ctx.ui
                        .send(TheEvent::SetStackIndex(TheId::named("Left Stack"), 1));
                    // if *SIDEBARMODE.lock().unwrap() != SidebarMode::Region {
                    //     if let Some(layout) = ui.get_shared_layout("Shared Panel Layout") {
                    //         layout.set_mode(TheSharedLayoutMode::Left);
                    //         ctx.ui.relayout = true;
                    //         redraw = true;
                    //     }
                    // }
                } else if id.name == "Set Tilemap Panel" {
                    //println!("Set Tilemap Panel");
                    ctx.ui
                        .send(TheEvent::SetStackIndex(TheId::named("Left Stack"), 2));
                    if let Some(layout) = ui.get_sharedhlayout("Shared Panel Layout") {
                        layout.set_mode(TheSharedHLayoutMode::Left);
                        ctx.ui.relayout = true;
                        redraw = true;
                    }
                }
            }
            _ => {}
        }

        redraw
    }

    pub fn update_code_object(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        server: &mut Server,
        server_ctx: &mut ServerContext,
    ) {
        fn create_items_for_value(
            object: &TheCodeObject,
            list: &mut dyn TheListLayoutTrait,
            ctx: &mut TheContext,
            indent: String,
        ) {
            for (name, value) in object.values.iter() {
                if let TheValue::CodeObject(object) = value {
                    create_items_for_value(object, list, ctx, str!("  ") + indent.as_str());
                } else if let TheValue::List(l) = value {
                    let mut item = TheListItem::new(TheId::empty());
                    item.set_text(indent.clone() + name.as_str());
                    item.add_value_column(120, value.clone());
                    list.add_item(item, ctx);

                    for v in l {
                        if let TheValue::CodeObject(object) = v {
                            create_items_for_value(object, list, ctx, str!("  ") + indent.as_str());
                        } else {
                            let mut item = TheListItem::new(TheId::empty());
                            item.set_text(indent.clone() + name.as_str());
                            item.add_value_column(120, value.clone());
                            list.add_item(item, ctx);
                        }
                    }
                } else {
                    let mut item = TheListItem::new(TheId::empty());
                    item.set_text(indent.clone() + name.as_str());
                    item.add_value_column(120, value.clone());
                    list.add_item(item, ctx);
                }
            }
        }

        if let Some(list) = ui.get_list_layout("CodeObject Layout") {
            list.clear();

            if let Some(character_id) = server_ctx.curr_character_instance {
                if let Some((object, _)) =
                    server.get_character_object(server_ctx.curr_region, character_id)
                {
                    create_items_for_value(&object, list, ctx, str!(""));
                }
            }
        }
    }

    /// Returns the current index of the editor group.
    fn get_editor_group_index(&self, ui: &mut TheUI) -> i32 {
        let mut index = 0;
        if let Some(widget) = ui.get_group_button("Editor Group") {
            index = widget.index();
        }
        index
    }
}
