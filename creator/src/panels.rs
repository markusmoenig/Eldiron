use crate::editor::{CODEEDITOR, SIDEBARMODE, TILEMAPEDITOR, TILEPICKER};
use crate::prelude::*;

pub struct Panels {}

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

        codeeditor.add_external(TheExternalCode::new(
            "RandWalk".to_string(),
            "Moves the character in a random direction.".to_string(),
            vec![],
            vec![],
            None,
        ));

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
            "Debug".to_string(),
            "Outputs the specified debug value.".to_string(),
            vec!["Value".to_string()],
            vec![TheValue::Text("Text".to_string())],
            None,
        ));

        Self {}
    }

    pub fn init_ui(&mut self, ui: &mut TheUI, ctx: &mut TheContext, _project: &mut Project) {
        let mut canvas = TheCanvas::new();

        //let mut tab_layout = TheTabLayout::new(TheId::named("Browser"));
        //tab_layout.limiter_mut().set_max_height(300);

        let mut shared_layout = TheSharedLayout::new(TheId::named("Shared Panel Layout"));
        shared_layout.limiter_mut().set_max_height(300);

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

        // Code Object details

        let mut codeobject_canvas = TheCanvas::new();
        let codeobject_layout = TheListLayout::new(TheId::named("CodeObject Layout"));
        codeobject_canvas.set_layout(codeobject_layout);

        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(vec4i(10, 2, 5, 2));

        let mut text = TheText::new(TheId::named("Panel Object Text"));
        text.set_text("Object".to_string());
        toolbar_hlayout.add_widget(Box::new(text));

        let mut toolbar_canvas = TheCanvas::default();
        toolbar_canvas.set_widget(TheTraybar::new(TheId::empty()));
        toolbar_canvas.set_layout(toolbar_hlayout);
        codeobject_canvas.set_top(toolbar_canvas);

        right_stack.add_canvas(codeobject_canvas);

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

        #[allow(clippy::single_match)]
        match event {
            TheEvent::Custom(id, _) => {
                if id.name == "Set Region Panel" {
                    //println!("Set Region Panel");

                    let mut shared_left = true;

                    if let Some(character) = server_ctx.curr_character_instance {
                        // Code Object
                        ctx.ui
                            .send(TheEvent::SetStackIndex(TheId::named("Right Stack"), 0));

                        // If in Pick mode show the instance
                        if self.get_editor_group_index(ui) == 1 {
                            ctx.ui
                                .send(TheEvent::SetStackIndex(TheId::named("Left Stack"), 1));

                            if let Some(layout) = ui.get_shared_layout("Shared Panel Layout") {
                                layout.set_mode(TheSharedLayoutMode::Shared);
                                layout.set_shared_ratio(0.7);
                                ctx.ui.relayout = true;
                                redraw = true;
                                shared_left = false;
                            }

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
                    } else {
                        ctx.ui
                            .send(TheEvent::SetStackIndex(TheId::named("Left Stack"), 0));
                    }

                    if shared_left {
                        if let Some(layout) = ui.get_shared_layout("Shared Panel Layout") {
                            layout.set_mode(TheSharedLayoutMode::Left);
                            ctx.ui.relayout = true;
                            redraw = true;
                        }
                    }
                } else if id.name == "Set CodeGrid Panel" {
                    //println!("Set CodeGrid Panel");
                    ctx.ui
                        .send(TheEvent::SetStackIndex(TheId::named("Left Stack"), 1));
                    if *SIDEBARMODE.lock().unwrap() != SidebarMode::Region {
                        if let Some(layout) = ui.get_shared_layout("Shared Panel Layout") {
                            layout.set_mode(TheSharedLayoutMode::Left);
                            ctx.ui.relayout = true;
                            redraw = true;
                        }
                    }
                } else if id.name == "Set Tilemap Panel" {
                    //println!("Set Tilemap Panel");
                    ctx.ui
                        .send(TheEvent::SetStackIndex(TheId::named("Left Stack"), 2));
                    if let Some(layout) = ui.get_shared_layout("Shared Panel Layout") {
                        layout.set_mode(TheSharedLayoutMode::Left);
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
        if let Some(list) = ui.get_list_layout("CodeObject Layout") {
            list.clear();

            if let Some(character_id) = server_ctx.curr_character_instance {
                if let Some((object, _)) =
                    server.get_character_object(server_ctx.curr_region, character_id)
                {
                    for (name, value) in object.values {
                        let mut item = TheListItem::new(TheId::empty());
                        item.set_text(name);
                        item.add_value_column(150, value);

                        list.add_item(item, ctx);
                    }
                }
            }
            /*
            let mut objects = Vec::new();

            for object in project.objects.values() {
                objects.push(object.clone());
            }

            objects.sort_by(|a, b| a.name.cmp(&b.name));

            for object in objects {
                let mut text = TheText::new(TheId::empty());
                text.set_text(object.name);
                list.add_widget(Box::new(text));
            }*/
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
