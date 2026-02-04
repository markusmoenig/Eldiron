use crate::prelude::*;
use coregridfxlib::{Module, Routine};
use std::sync::mpsc::Receiver;

pub struct CodeEditor {
    module: Module,

    event_receiver: Option<Receiver<TheEvent>>,
}

impl TheTrait for CodeEditor {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            module: Module::new("New Module"),
            event_receiver: None,
        }
    }

    fn window_title(&self) -> String {
        "CodeGridFX".to_string()
    }

    fn init_ui(&mut self, ui: &mut TheUI, _ctx: &mut TheContext) {
        /*
        // Menu
        let mut menu_canvas = TheCanvas::new();
        let mut menu = TheMenu::new(TheId::named("Menu"));

        let mut file_menu = TheContextMenu::named(str!("File"));
        file_menu.add(TheContextMenuItem::new(
            str!("Open..."),
            TheId::named("Open"),
        ));
        file_menu.add(TheContextMenuItem::new(str!("Save"), TheId::named("Save")));
        file_menu.add(TheContextMenuItem::new(
            str!("Save As ..."),
            TheId::named("Save As"),
        ));
        let mut edit_menu = TheContextMenu::named(str!("Edit"));
        edit_menu.add(TheContextMenuItem::new(str!("Undo"), TheId::named("Undo")));
        edit_menu.add(TheContextMenuItem::new(str!("Redo"), TheId::named("Redo")));
        edit_menu.add_separator();
        edit_menu.add(TheContextMenuItem::new(str!("Cut"), TheId::named("Cut")));
        edit_menu.add(TheContextMenuItem::new(str!("Copy"), TheId::named("Copy")));
        edit_menu.add(TheContextMenuItem::new(
            str!("Paste"),
            TheId::named("Paste"),
        ));

        let mut code_menu = TheContextMenu::named(str!("Code"));
        code_menu.add(self.editor.create_keywords_context_menu_item());
        code_menu.add(self.editor.create_operators_context_menu_item());
        code_menu.add(self.editor.create_values_context_menu_item());
        code_menu.add(self.editor.create_functions_context_menu_item());

        menu.add_context_menu(file_menu);
        menu.add_context_menu(edit_menu);
        menu.add_context_menu(code_menu);

        menu_canvas.set_widget(menu);

        self.editor.init_menu_selection(ctx);
        */
        // Top
        let mut top_canvas = TheCanvas::new();

        let mut menubar = TheMenubar::new(TheId::named("Menubar"));
        menubar.limiter_mut().set_max_height(43 + 22);

        let mut open_button = TheMenubarButton::new(TheId::named("Open"));
        open_button.set_icon_name("icon_role_load".to_string());

        let mut save_button = TheMenubarButton::new(TheId::named("Save"));
        save_button.set_icon_name("icon_role_save".to_string());

        let mut save_as_button = TheMenubarButton::new(TheId::named("Save As"));
        save_as_button.set_icon_name("icon_role_save_as".to_string());
        save_as_button.set_icon_offset(Vec2::new(2, -5));

        let mut undo_button = TheMenubarButton::new(TheId::named("Undo"));
        undo_button.set_icon_name("icon_role_undo".to_string());

        let mut redo_button = TheMenubarButton::new(TheId::named("Redo"));
        redo_button.set_icon_name("icon_role_redo".to_string());

        let mut hlayout = TheHLayout::new(TheId::named("Menu Layout"));
        hlayout.set_background_color(None);
        hlayout.set_margin(Vec4::new(40, 5, 20, 0));
        hlayout.add_widget(Box::new(open_button));
        hlayout.add_widget(Box::new(save_button));
        hlayout.add_widget(Box::new(save_as_button));
        hlayout.add_widget(Box::new(TheMenubarSeparator::new(TheId::empty())));
        hlayout.add_widget(Box::new(undo_button));
        hlayout.add_widget(Box::new(redo_button));

        top_canvas.set_widget(menubar);
        top_canvas.set_layout(hlayout);
        // top_canvas.set_top(menu_canvas);

        /*
        // Side

        let bundle_canvas =
            self.editor
                .set_bundle(self.project.bundle.clone(), ctx, self.right_width, None);
        ui.canvas.set_right(bundle_canvas);

        let mut status_canvas = TheCanvas::new();
        let mut statusbar = TheStatusbar::new(TheId::named("Statusbar"));
        statusbar.set_text("Welcome to TheFramework!".to_string());
        status_canvas.set_widget(statusbar);

        //

        ui.canvas.set_top(top_canvas);
        ui.canvas.set_bottom(status_canvas);
        ui.canvas.set_center(self.editor.build_canvas(ctx));
        ui.set_statusbar_name("Statusbar".to_string());

        ctx.ui.set_disabled("Save");
        ctx.ui.set_disabled("Save As");
        ctx.ui.set_disabled("Undo");
        ctx.ui.set_disabled("Redo");
        */

        self.module.get_colors(ui);
        let startup_routine = Routine::new("Startup".into());
        self.module.add_routine(startup_routine);

        ui.canvas.set_center(self.module.build_canvas());

        self.event_receiver = Some(ui.add_state_listener("Main Receiver".into()));
    }

    fn update_ui(&mut self, ui: &mut TheUI, ctx: &mut TheContext) -> bool {
        let mut redraw: bool = false;

        if let Some(receiver) = &mut self.event_receiver {
            while let Ok(event) = receiver.try_recv() {
                redraw = self.module.handle_event(&event, ui, ctx);
                // match event {
                //     TheEvent::WidgetResized(id, dim) => {
                //         println!("{:?} {:?}", id, dim);

                //         if id.name == "ModuleView" {
                //             if let Some(renderview) = ui.get_render_view("ModuleView") {
                //                 *renderview.render_buffer_mut() =
                //                     TheRGBABuffer::new(TheDim::new(0, 0, dim.width, dim.height));
                //                 renderview.render_buffer_mut().fill(BLACK);
                //             }
                //         }
                //     }
                //     _ => {}
                // }
            }
        }
        /*
        if let Some(receiver) = &mut self.event_receiver {
            while let Ok(event) = receiver.try_recv() {
                redraw = self.editor.handle_event(&event, ui, ctx);
                match event {
                    TheEvent::ContextMenuSelected(_, action) => {
                        if action.name.starts_with("Code") {
                            self.editor.insert_context_menu_id(action, ui, ctx);
                        }
                    }
                    TheEvent::FileRequesterResult(id, paths) => {
                        if id.name == "Open" {
                            for p in paths {
                                self.project_path = Some(p.clone());
                                let contents = std::fs::read_to_string(p).unwrap_or("".to_string());
                                self.project =
                                    serde_json::from_str(&contents).unwrap_or(Project::new());
                                ui.canvas.set_right(self.editor.set_bundle(
                                    self.project.bundle.clone(),
                                    ctx,
                                    self.right_width,
                                    None,
                                ));
                                redraw = true;
                                ctx.ui.send(TheEvent::SetStatusText(
                                    TheId::empty(),
                                    "Project loaded successfully.".to_string(),
                                ))
                            }
                        } else if id.name == "Save As" {
                            self.project.bundle = self.editor.get_bundle();
                            for p in paths {
                                let json = serde_json::to_string(&self.project);
                                if let Ok(json) = json {
                                    if std::fs::write(p, json).is_ok() {
                                        ctx.ui.send(TheEvent::SetStatusText(
                                            TheId::empty(),
                                            "Project saved successfully.".to_string(),
                                        ))
                                    } else {
                                        ctx.ui.send(TheEvent::SetStatusText(
                                            TheId::empty(),
                                            "Unable to save project!".to_string(),
                                        ))
                                    }
                                }
                            }
                        }
                    }
                    TheEvent::CodeBundleChanged(_, _) => {
                        redraw = true;
                    }
                    TheEvent::StateChanged(id, _state) => {
                        if id.name == "Open" {
                            #[cfg(not(target_arch = "wasm32"))]
                            ctx.ui.open_file_requester(
                                TheId::named_with_id(id.name.as_str(), Uuid::new_v4()),
                                "Open".into(),
                                TheFileExtension::new(
                                    "CodeGridFX".into(),
                                    vec!["codegridfx".to_string()],
                                ),
                            );
                            ctx.ui
                                .set_widget_state("Open".to_string(), TheWidgetState::None);
                            ctx.ui.clear_hover();
                            redraw = true;
                        } else if id.name == "Save" {
                            self.project.bundle = self.editor.get_bundle();
                            if let Some(path) = &self.project_path {
                                let json = serde_json::to_string(&self.project);
                                if let Ok(json) = json {
                                    if std::fs::write(path, json).is_ok() {
                                        ctx.ui.send(TheEvent::SetStatusText(
                                            TheId::empty(),
                                            "Project saved successfully.".to_string(),
                                        ))
                                    } else {
                                        ctx.ui.send(TheEvent::SetStatusText(
                                            TheId::empty(),
                                            "Unable to save project!".to_string(),
                                        ))
                                    }
                                }
                            }
                        } else if id.name == "Save As" {
                            #[cfg(not(target_arch = "wasm32"))]
                            ctx.ui.save_file_requester(
                                TheId::named_with_id(id.name.as_str(), Uuid::new_v4()),
                                "Save".into(),
                                TheFileExtension::new(
                                    "CodeGridFX".into(),
                                    vec!["codegridfx".to_string()],
                                ),
                            );
                            ctx.ui
                                .set_widget_state("Save".to_string(), TheWidgetState::None);
                            ctx.ui.clear_hover();
                            redraw = true;
                        } else if id.name == "Compile" {
                            if let Some(layout) = ui.get_code_layout("Code Editor") {
                                if let Some(code_view) = layout.code_view_mut().as_code_view() {
                                    let grid = code_view.codegrid_mut();

                                    let rc = self.compiler.compile(grid);

                                    if let Ok(mut module) = rc {
                                        let mut sandbox = TheCodeSandbox::new();
                                        sandbox.debug_mode = true;

                                        // sandbox.add_global(
                                        //     "test",
                                        //     TheCodeNode::new(
                                        //         |_, data, _| {
                                        //             println!("inside test {:?}", data.location);
                                        //             if let Some(i) = data.values[0].to_i32() {
                                        //                 println!("i: {:?}", i);
                                        //                 data.values[0] = TheValue::Int(i + 1);
                                        //             }
                                        //             TheCodeNodeCallResult::Continue
                                        //         },
                                        //         TheCodeNodeData::values(vec![TheValue::Int(0)]),
                                        //     ),
                                        //     vec![TheCodeAtom::NamedValue("Count".to_string(), TheValue::Int(4))]
                                        // );

                                        //sandbox.insert_module(module.clone());
                                        module.execute(&mut sandbox);
                                        code_view.set_debug_module(
                                            sandbox.get_module_debug_module(module.id),
                                        );
                                    } else {
                                        code_view.set_debug_module(TheDebugModule::default());
                                    }
                                }
                            }
                        } else {
                            let mut data: Option<(TheId, String)> = None;
                            if id.name == "Undo" && ctx.ui.undo_stack.has_undo() {
                                data = Some(ctx.ui.undo_stack.undo());
                            } else if id.name == "Redo" && ctx.ui.undo_stack.has_redo() {
                                data = Some(ctx.ui.undo_stack.redo());
                            }

                            if let Some((id, json)) = data {
                                if id.name == "Code Editor" {
                                    self.editor.set_codegrid_json(json, ui);
                                    self.editor.set_grid_selection_ui(ui, ctx);
                                }
                                redraw = true;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }*/
        redraw
    }
}
