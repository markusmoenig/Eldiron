use crate::editor::{
    CODEEDITOR, MODELEDITOR, REGIONFXEDITOR, TILEDRAWER, TILEFXEDITOR, TILEMAPEDITOR, TILEPICKER,
};
use crate::prelude::*;

pub struct Panels {
    pub curr_atom: Option<TheCodeAtom>,
    pub tilefx_visible: bool,
}

#[allow(clippy::new_without_default)]
impl Panels {
    pub fn new() -> Self {
        Self {
            curr_atom: None,
            tilefx_visible: false,
        }
    }

    pub fn init_ui(
        &mut self,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        _project: &mut Project,
        _server_ctx: &mut ServerContext,
    ) -> TheCanvas {
        let mut canvas = TheCanvas::new();

        let mut shared_layout = TheSharedHLayout::new(TheId::named("Shared Panel Layout"));
        shared_layout.set_shared_ratio(0.27);
        shared_layout.set_mode(TheSharedHLayoutMode::Right);

        // Main Stack

        let mut main_canvas = TheCanvas::new();
        let mut main_stack = TheStackLayout::new(TheId::named("Main Stack"));

        main_stack.add_canvas(TILEPICKER.lock().unwrap().build(false));
        main_stack.add_canvas(CODEEDITOR.lock().unwrap().build_canvas(ctx));
        main_stack.add_canvas(TILEMAPEDITOR.lock().unwrap().build());
        main_stack.add_canvas(TILEFXEDITOR.lock().unwrap().build(ctx));
        // main_stack.add_canvas(MODELFXEDITOR.lock().unwrap().build_mapobjects(ctx));
        // main_stack.add_canvas(
        //     MODELFXEDITOR
        //         .lock()
        //         .unwrap()
        //         .build_brush_ui(project, ctx, server_ctx),
        // );
        main_stack.add_canvas(MODELEDITOR.lock().unwrap().build_node_ui());

        // let mut code_canvas = TheCanvas::new();
        // let mut widget = TheTextAreaEdit::new(TheId::named("Text"));
        // widget.set_value(TheValue::Text("Your Code".to_string()));
        // // If we ignore code type, it's a plain text edit area
        // widget.set_code_type("glsl");
        // code_canvas.set_widget(widget);
        // main_stack.add_canvas(code_canvas);
        main_stack.add_canvas(REGIONFXEDITOR.lock().unwrap().build(ctx));

        main_stack.set_index(0);

        let tilemap_editor = TheRGBALayout::new(TheId::named("Tilemap Editor"));
        let mut tilemap_canvas = TheCanvas::new();
        tilemap_canvas.set_layout(tilemap_editor);
        main_stack.add_canvas(tilemap_canvas);

        main_canvas.set_layout(main_stack);

        // Details Stack

        let mut details_canvas = TheCanvas::new();
        let mut details_stack = TheStackLayout::new(TheId::named("Details Stack"));

        // Context Group

        let mut context_group: TheGroupButton =
            TheGroupButton::new(TheId::named("Details Stack Group"));
        context_group.add_text_status(
            "Context".to_string(),
            "Shows the visual context of the selected code.".to_string(),
        );
        context_group.add_text_status(
            "Object".to_string(),
            "Shows the object properties.".to_string(),
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
        details_canvas.set_top(toolbar_canvas);

        // Context

        let mut codecontext_canvas = TheCanvas::new();
        let codecontext_layout = TheListLayout::new(TheId::named("CodeObject Context Layout"));
        codecontext_canvas.set_layout(codecontext_layout);

        details_stack.add_canvas(codecontext_canvas);

        // Object

        let mut codeobject_canvas = TheCanvas::new();
        let codeobject_layout = TheListLayout::new(TheId::named("CodeObject Layout"));
        codeobject_canvas.set_layout(codeobject_layout);

        details_stack.add_canvas(codeobject_canvas);

        // Out

        let mut out_canvas = TheCanvas::new();

        let codeobject_layout = TheListLayout::new(TheId::named("CodeObject Output Layout"));
        out_canvas.set_layout(codeobject_layout);

        details_stack.add_canvas(out_canvas);

        //

        details_canvas.set_layout(details_stack);

        //

        shared_layout.add_canvas(details_canvas);
        shared_layout.add_canvas(main_canvas);

        canvas.set_layout(shared_layout);

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
        let mut redraw = CODEEDITOR.lock().unwrap().handle_event(event, ui, ctx);
        if TILEPICKER
            .lock()
            .unwrap()
            .handle_event(event, ui, ctx, project, server)
        {
            redraw = true;
        }

        match event {
            TheEvent::StateChanged(id, state) => {
                if (id.name == "Ground Icon" || id.name == "Wall Icon" || id.name == "Ceiling Icon")
                    && *state == TheWidgetState::Clicked
                {
                    if self.tilefx_visible {
                        self.tilefx_visible = false;
                    }
                } else if id.name == "Tile FX Icon"
                    && *state == TheWidgetState::Clicked
                    && !self.tilefx_visible
                {
                    self.tilefx_visible = true;
                }
            }
            TheEvent::CodeEditorSelectionChanged(_, _) | TheEvent::CodeBundleChanged(_, _) => {
                let mut set_to = TheCanvas::new();
                let mut set_already = false;

                if let Some(atom) = CODEEDITOR.lock().unwrap().get_selected_atom(ui) {
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
                    if let TheCodeAtom::Value(TheValue::ColorObject(color)) = &atom {
                        let mut vlayout = TheVLayout::new(TheId::empty());

                        let mut w = TheColorPicker::new(TheId::named("Atom Color Picker"));
                        w.set_value(TheValue::ColorObject(color.clone()));
                        vlayout.set_background_color(Some(ListLayoutBackground));
                        vlayout.set_margin(vec4i(20, 20, 20, 20));
                        vlayout.add_widget(Box::new(w));
                        set_to.set_layout(vlayout);
                        set_already = true;
                    }
                    if let TheCodeAtom::Value(TheValue::Direction(value)) = &atom {
                        let mut vlayout = TheVLayout::new(TheId::empty());

                        let mut w = TheDirectionPicker::new(TheId::named("Atom Direction Picker"));
                        w.set_value(TheValue::Direction(*value));
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
                        .send(TheEvent::SetStackIndex(TheId::named("Details Stack"), 0));
                    ui.set_widget_value("Details Stack Group", ctx, TheValue::Int(0));
                }

                if let Some(stack) = ui.get_stack_layout("Details Stack") {
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
                if id.name == "Details Stack Group" {
                    if let Some(stack) = ui.get_stack_layout("Details Stack") {
                        stack.set_index(*index);
                        redraw = true;
                        ctx.ui.relayout = true;
                    }
                }
            }
            TheEvent::Custom(id, _) => {
                if id.name == "Set Region Modeler" {
                    ctx.ui
                        .send(TheEvent::SetStackIndex(TheId::named("Main Stack"), 4));
                    if let Some(layout) = ui.get_sharedhlayout("Shared Panel Layout") {
                        layout.set_mode(TheSharedHLayoutMode::Right);
                        ctx.ui.relayout = true;
                        redraw = true;
                    }
                    // MODELFXEDITOR
                    //     .lock()
                    //     .unwrap()
                    //     .activated(server_ctx, project, ui, ctx);
                } else if id.name == "Set Region Brush" {
                    ctx.ui
                        .send(TheEvent::SetStackIndex(TheId::named("Main Stack"), 5));
                    if let Some(layout) = ui.get_sharedhlayout("Shared Panel Layout") {
                        layout.set_mode(TheSharedHLayoutMode::Right);
                        ctx.ui.relayout = true;
                        redraw = true;
                    }
                    // MODELFXEDITOR
                    //     .lock()
                    //     .unwrap()
                    //     .activated(server_ctx, project, ui, ctx);
                } else if id.name == "Set Region Render" {
                    ctx.ui
                        .send(TheEvent::SetStackIndex(TheId::named("Main Stack"), 7));
                    if let Some(layout) = ui.get_sharedhlayout("Shared Panel Layout") {
                        layout.set_mode(TheSharedHLayoutMode::Right);
                        ctx.ui.relayout = true;
                        redraw = true;
                    }
                }
                // else if id.name == "Set Region Panel" {
                //     let mut shared_left = true;

                //     if let Some(character) = server_ctx.curr_character_instance {
                //         // Character
                //         ctx.ui
                //             .send(TheEvent::SetStackIndex(TheId::named("Details Stack"), 1));
                //         ui.set_widget_value("Details Stack Group", ctx, TheValue::Int(1));

                //         shared_left = false;

                //         // If in Pick mode show the instance
                //         if self.get_editor_group_index(ui) == EditorMode::Pick as i32 {
                //             ctx.ui
                //                 .send(TheEvent::SetStackIndex(TheId::named("Main Stack"), 1));

                //             if let Some(layout) = ui.get_sharedhlayout("Shared Panel Layout") {
                //                 layout.set_mode(TheSharedHLayoutMode::Shared);
                //                 ctx.ui.relayout = true;
                //                 redraw = true;
                //             }

                //             ui.set_widget_value("Details Stack Group", ctx, TheValue::Int(1));

                //             if let Some((name, _)) = server.get_character_property(
                //                 server_ctx.curr_region,
                //                 character,
                //                 "name".into(),
                //             ) {
                //                 if let Some(text) = ui.get_text("Panel Object Text") {
                //                     text.set_text(name.describe());
                //                 }
                //             }

                //             self.update_code_object(ui, ctx, server, server_ctx);
                //         }
                //     } else if let Some(item) = server_ctx.curr_item_instance {
                //         // Item
                //         ctx.ui
                //             .send(TheEvent::SetStackIndex(TheId::named("Details Stack"), 1));
                //         ui.set_widget_value("Details Stack Group", ctx, TheValue::Int(1));

                //         shared_left = false;

                //         // If in Pick mode show the instance
                //         if self.get_editor_group_index(ui) == EditorMode::Pick as i32 {
                //             ctx.ui
                //                 .send(TheEvent::SetStackIndex(TheId::named("Main Stack"), 1));

                //             if let Some(layout) = ui.get_sharedhlayout("Shared Panel Layout") {
                //                 layout.set_mode(TheSharedHLayoutMode::Shared);
                //                 ctx.ui.relayout = true;
                //                 redraw = true;
                //             }

                //             ui.set_widget_value("Details Stack Group", ctx, TheValue::Int(1));

                //             if let Some((name, _)) = server.get_item_property(
                //                 server_ctx.curr_region,
                //                 item,
                //                 "name".into(),
                //             ) {
                //                 if let Some(text) = ui.get_text("Panel Object Text") {
                //                     text.set_text(name.describe());
                //                 }
                //             }

                //             self.update_code_object(ui, ctx, server, server_ctx);
                //         }
                //     } else if let Some(area_id) = server_ctx.curr_area {
                //         // Area
                //         ctx.ui
                //             .send(TheEvent::SetStackIndex(TheId::named("Details Stack"), 1));

                //         // If in Pick mode show the instance
                //         if self.get_editor_group_index(ui) == EditorMode::Pick as i32 {
                //             ctx.ui
                //                 .send(TheEvent::SetStackIndex(TheId::named("Main Stack"), 1));

                //             if let Some(layout) = ui.get_sharedhlayout("Shared Panel Layout") {
                //                 layout.set_mode(TheSharedHLayoutMode::Shared);
                //                 ctx.ui.relayout = true;
                //                 redraw = true;
                //                 shared_left = false;
                //             }

                //             ui.set_widget_value("Details Stack Group", ctx, TheValue::Int(1));

                //             if let Some(region) = project.get_region(&server_ctx.curr_region) {
                //                 if let Some(area) = region.areas.get(&area_id) {
                //                     if let Some(text) = ui.get_text("Panel Object Text") {
                //                         text.set_text(area.name.clone());
                //                     }
                //                 }
                //             }

                //             self.update_code_object(ui, ctx, server, server_ctx);
                //         }
                //     } else if !self.tilefx_visible {
                //         // Tile Picker
                //         let editor_group_index = self.get_editor_group_index(ui);
                //         if editor_group_index == EditorMode::Draw as i32 {
                //             ctx.ui
                //                 .send(TheEvent::SetStackIndex(TheId::named("Main Stack"), 0));
                //         } else if editor_group_index == EditorMode::Model as i32 {
                //             ctx.ui
                //                 .send(TheEvent::SetStackIndex(TheId::named("Main Stack"), 4));
                //         } else if editor_group_index == EditorMode::Render as i32 {
                //             ctx.ui
                //                 .send(TheEvent::SetStackIndex(TheId::named("Main Stack"), 5));
                //         }
                //     } else {
                //         // Tile CC
                //         ctx.ui
                //             .send(TheEvent::SetStackIndex(TheId::named("Main Stack"), 3));
                //     }

                //     if shared_left {
                //         if let Some(layout) = ui.get_sharedhlayout("Shared Panel Layout") {
                //             layout.set_mode(TheSharedHLayoutMode::Right);
                //             ctx.ui.relayout = true;
                //             redraw = true;
                //         }
                //     }
                else if id.name == "Set CodeGrid Panel" {
                    ctx.ui
                        .send(TheEvent::SetStackIndex(TheId::named("Main Stack"), 1));
                    if let Some(layout) = ui.get_sharedhlayout("Shared Panel Layout") {
                        layout.set_mode(TheSharedHLayoutMode::Shared);
                        ctx.ui.relayout = true;
                        redraw = true;
                    }
                } else if id.name == "Set Tilemap Panel" {
                    ctx.ui
                        .send(TheEvent::SetStackIndex(TheId::named("Main Stack"), 2));
                    if let Some(layout) = ui.get_sharedhlayout("Shared Panel Layout") {
                        layout.set_mode(TheSharedHLayoutMode::Right);
                        ctx.ui.relayout = true;
                        redraw = true;
                    }
                } else if id.name == "Set Tilepicker Panel" {
                    ctx.ui
                        .send(TheEvent::SetStackIndex(TheId::named("Main Stack"), 0));
                    if let Some(layout) = ui.get_sharedhlayout("Shared Panel Layout") {
                        layout.set_mode(TheSharedHLayoutMode::Right);
                        ctx.ui.relayout = true;
                        redraw = true;
                    }
                }
            }
            _ => {}
        }

        redraw
    }

    /// Sets the brush panel.
    pub fn set_brush_panel(&mut self, ui: &mut TheUI, ctx: &mut TheContext) {
        if let Some(stack) = ui.get_stack_layout("Main Stack") {
            stack.set_index(5);
        }
        if let Some(layout) = ui.get_sharedhlayout("Shared Panel Layout") {
            layout.set_mode(TheSharedHLayoutMode::Right);
            ctx.ui.relayout = true;
        }
    }

    /// Updates the code object, i.e. displays the object properties and interactions.
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
            inside_list: bool,
        ) {
            for (name, value) in object.values.iter() {
                if let TheValue::CodeObject(object) = value {
                    create_items_for_value(
                        object,
                        list,
                        ctx,
                        str!("  ") + indent.as_str(),
                        inside_list,
                    );
                } else if let TheValue::List(l) = value {
                    let mut item = TheListItem::new(TheId::empty());
                    item.set_text(indent.clone() + name.as_str());
                    item.add_value_column(120, value.clone());
                    item.set_background_color(TheColor::from_hex("#e0c872"));
                    list.add_item(item, ctx);

                    for v in l {
                        if let TheValue::CodeObject(object) = v {
                            if let Some(name) = object.get(&str!("name")) {
                                let mut item = TheListItem::new(TheId::empty());
                                item.set_text(indent.clone() + &indent + "  " + &name.describe());
                                item.add_value_column(120, TheValue::Text("Object".into()));
                                item.set_background_color(TheColor::from_hex("#d4804d"));
                                list.add_item(item, ctx);
                            }

                            // create_items_for_value(
                            //     object,
                            //     list,
                            //     ctx,
                            //     str!("  ") + indent.as_str(),
                            //     true,
                            // );
                        } else {
                            let mut item = TheListItem::new(TheId::empty());
                            item.set_text(indent.clone() + name.as_str());
                            item.set_background_color(TheColor::from_hex("#e0c872"));
                            item.add_value_column(120, value.clone());
                            list.add_item(item, ctx);
                        }
                    }
                } else if !name.starts_with('_') {
                    let mut item = TheListItem::new(TheId::empty());
                    item.set_text(indent.clone() + name.as_str());
                    item.add_value_column(120, value.clone());
                    if inside_list {
                        item.set_background_color(TheColor::from_hex("#e0c872"));
                    }
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
                    create_items_for_value(&object, list, ctx, str!(""), false);
                }
            }
        }

        if let Some(list) = ui.get_list_layout("CodeObject Output Layout") {
            list.clear();

            if let Some(character_id) = server_ctx.curr_character_instance {
                if let Some(interaction_list) = server_ctx.interactions.get(&character_id) {
                    for interaction in interaction_list {
                        let mut item = TheListItem::new(TheId::empty());
                        item.set_text(interaction.value.describe());
                        item.set_status_text(&interaction.value.describe());
                        //item.set_background_color(TheColor::from_hex("#e0c872"));
                        item.add_value_column(100, TheValue::Text(interaction.from_name.clone()));
                        list.add_item(item, ctx);
                    }
                }
            }
        }
    }

    // Returns the current index of the editor group.
    // fn get_editor_group_index(&self, ui: &mut TheUI) -> i32 {
    //     let mut index = 0;
    //     if let Some(widget) = ui.get_group_button("Editor Group") {
    //         index = widget.index();
    //     }
    //     index
    // }
}
