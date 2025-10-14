use crate::{
    editor::{CODEEDITOR, CODEGRIDFX},
    prelude::*,
};
use ToolEvent::*;

pub struct CodeTool {
    id: TheId,
    use_python: bool,
}

impl Tool for CodeTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Code Tool"),
            use_python: false,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }
    fn info(&self) -> String {
        str!("Code Tool (C).")
    }
    fn icon_name(&self) -> String {
        str!("code")
    }
    fn accel(&self) -> Option<char> {
        Some('C')
    }

    fn tool_event(
        &mut self,
        tool_event: ToolEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        _project: &mut Project,
        _server_ctx: &mut ServerContext,
    ) -> bool {
        if let Activate = tool_event {
            CODEEDITOR.write().unwrap().active_panel = VisibleCodePanel::Code;

            if !self.use_python {
                ctx.ui.send(TheEvent::SetStackIndex(
                    TheId::named("Main Stack"),
                    PanelIndices::CodeGridFx as usize,
                ));

                if let Some(renderview) = ui.get_render_view("CodeModuleView") {
                    *renderview.render_buffer_mut() = TheRGBABuffer::new(TheDim::new(
                        0,
                        0,
                        renderview.dim().width,
                        renderview.dim().height,
                    ));
                    CODEGRIDFX
                        .write()
                        .unwrap()
                        .draw(renderview.render_buffer_mut());
                }
            } else {
                ctx.ui.send(TheEvent::SetStackIndex(
                    TheId::named("Main Stack"),
                    PanelIndices::CodeEditor as usize,
                ));
            }

            if let Some(layout) = ui.get_hlayout("Game Tool Params") {
                layout.clear();

                let mut code_switch = TheGroupButton::new(TheId::named("Code Node Switch"));
                code_switch.add_text_status(
                    "Nodes".to_string(),
                    str!("Use nodes to create Python code."),
                );
                code_switch
                    .add_text_status("Python".to_string(), "Code directly in Python.".to_string());
                code_switch.set_index(if self.use_python { 1 } else { 0 });
                code_switch.set_item_width(100);
                layout.add_widget(Box::new(code_switch));

                let mut hdivider = TheHDivider::new(TheId::empty());
                hdivider.limiter_mut().set_max_width(15);
                layout.add_widget(Box::new(hdivider));

                let mut text = TheText::new(TheId::named("Code Editor Header Text"));
                text.set_text(CODEEDITOR.read().unwrap().last_header_text.clone());
                layout.add_widget(Box::new(text));

                let mut template_switch = TheGroupButton::new(TheId::named("Code Template Switch"));
                template_switch.add_text_status(
                    "Template".to_string(),
                    str!("Show the character / item template code."),
                );
                template_switch.add_text_status(
                    "Instance".to_string(),
                    "Show the character / item instantiation code.".to_string(),
                );
                template_switch.set_index(if CODEEDITOR.read().unwrap().show_template {
                    0
                } else {
                    1
                });
                template_switch.set_item_width(100);
                layout.add_widget(Box::new(template_switch));

                /*
                let mut spaces_switch = TheGroupButton::new(TheId::named("Code Spaces Switch"));
                spaces_switch.add_text_status(
                    "Show Spaces".to_string(),
                    str!("Visually display spaces in the editor."),
                );
                spaces_switch.add_text_status("Hide".to_string(), "Hide spaces.".to_string());
                spaces_switch.set_index(0);
                spaces_switch.set_item_width(100);
                layout.add_widget(Box::new(spaces_switch));
                */

                layout.set_reverse_index(Some(1));
            }

            return true;
        };

        false
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let mut redraw = false;

        if !self.use_python {
            CODEGRIDFX
                .write()
                .unwrap()
                .handle_event(event, ui, ctx, &project.palette);
        }

        match event {
            TheEvent::IndexChanged(id, index) => {
                if id.name == "Code Node Switch" {
                    if *index == 0 {
                        self.use_python = false;
                    } else {
                        self.use_python = true;
                    }

                    if !self.use_python {
                        ctx.ui.send(TheEvent::SetStackIndex(
                            TheId::named("Main Stack"),
                            PanelIndices::CodeGridFx as usize,
                        ));
                    } else {
                        ctx.ui.send(TheEvent::SetStackIndex(
                            TheId::named("Main Stack"),
                            PanelIndices::CodeEditor as usize,
                        ));
                    }
                }

                // if id.name == "Code Spaces Switch" {
                //     if let Some(edit) = ui.get_text_area_edit("CodeEdit") {
                //         edit.as_code_editor(
                //             "Python",
                //             TheCodeEditorSettings {
                //                 auto_bracket_completion: true,
                //                 auto_indent: true,
                //                 indicate_space: *index == 0,
                //             },
                //         );
                //     }
                // }

                if id.name == "Code Template Switch" {
                    CODEEDITOR.write().unwrap().switch_module_to(
                        ui,
                        ctx,
                        project,
                        server_ctx,
                        *index == 0,
                    );
                }
            }
            TheEvent::StateChanged(id, state) => {
                #[allow(clippy::collapsible_if)]
                if id.name == "Build" && *state == TheWidgetState::Clicked {
                    if self.use_python == false {
                        // Build the node code.
                        let code = CODEGRIDFX.read().unwrap().build(false);
                        let debug_code = CODEGRIDFX.read().unwrap().build(true);
                        ui.set_widget_value("CodeEdit", ctx, TheValue::Text(code.clone()));
                        match server_ctx.cc {
                            ContentContext::CharacterInstance(uuid) => {
                                if let Some(region) =
                                    project.get_region_mut(&server_ctx.curr_region)
                                {
                                    if let Some(character_instance) =
                                        region.characters.get_mut(&uuid)
                                    {
                                        character_instance.source = code;
                                        character_instance.source_debug = debug_code;
                                    }
                                }
                            }
                            ContentContext::CharacterTemplate(uuid) => {
                                if let Some(character) = project.characters.get_mut(&uuid) {
                                    character.source = code;
                                    character.source_debug = debug_code;
                                }
                            }
                            ContentContext::ItemTemplate(uuid) => {
                                if let Some(item) = project.items.get_mut(&uuid) {
                                    item.source = code;
                                    item.source_debug = debug_code;
                                }
                            }
                            _ => {}
                        }
                    } else if let Some(value) = ui.get_widget_value("CodeEdit") {
                        if let Some(code) = value.to_string() {
                            // Compile the code to test for errors.
                            let ri = rusterix::RegionInstance::new(0);
                            match ri.execute(&code) {
                                Ok(_) => {
                                    ctx.ui.send(TheEvent::SetStatusText(
                                        TheId::empty(),
                                        "Build OK".to_string(),
                                    ));
                                    // ui.set_widget_value(
                                    //     "Build Result",
                                    //     ctx,
                                    //     TheValue::Text("Build OK".into()),
                                    // );
                                }
                                Err(err) => {
                                    ctx.ui.send(TheEvent::SetStatusText(
                                        TheId::empty(),
                                        format!("Error: {err}"),
                                    ));
                                    // ui.set_widget_value(
                                    //     "Build Result",
                                    //     ctx,
                                    //     TheValue::Text(format!("Error: {err}")),
                                    // );
                                }
                            }
                            if let Some(layout) = ui.get_hlayout("Game Tool Params") {
                                layout.relayout(ctx);
                            }
                        }
                    }
                }
            }
            TheEvent::Custom(id, _) => {
                if id.name == "ModuleChanged"
                    && CODEEDITOR.read().unwrap().active_panel == VisibleCodePanel::Code
                {
                    let code = CODEGRIDFX.read().unwrap().build(false);
                    let debug_code = CODEGRIDFX.read().unwrap().build(true);
                    // println!("{}", debug_code);
                    ui.set_widget_value("CodeEdit", ctx, TheValue::Text(code.clone()));

                    match CODEEDITOR.read().unwrap().code_content {
                        ContentContext::CharacterInstance(uuid) => {
                            if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                                if let Some(character_instance) = region.characters.get_mut(&uuid) {
                                    character_instance.module = CODEGRIDFX.read().unwrap().clone();
                                    character_instance.source = code;
                                    character_instance.source_debug = debug_code;
                                }
                            }
                        }
                        ContentContext::CharacterTemplate(uuid) => {
                            if let Some(character) = project.characters.get_mut(&uuid) {
                                character.module = CODEGRIDFX.read().unwrap().clone();
                                character.source = code;
                                character.source_debug = debug_code;
                            }
                        }
                        ContentContext::ItemTemplate(uuid) => {
                            if let Some(item) = project.items.get_mut(&uuid) {
                                item.module = CODEGRIDFX.read().unwrap().clone();
                                item.source = code;
                                item.source_debug = debug_code;
                            }
                        }
                        _ => {}
                    }
                }
            }
            TheEvent::ValueChanged(id, value) => {
                if id.name == "CodeEdit" {
                    if let Some(code) = value.to_string() {
                        match server_ctx.cc {
                            ContentContext::CharacterInstance(uuid) => {
                                if let Some(region) =
                                    project.get_region_mut(&server_ctx.curr_region)
                                {
                                    if let Some(character_instance) =
                                        region.characters.get_mut(&uuid)
                                    {
                                        character_instance.source = code;
                                        character_instance.source_debug = "".into();
                                    }
                                }
                            }
                            ContentContext::CharacterTemplate(uuid) => {
                                if let Some(character) = project.characters.get_mut(&uuid) {
                                    let class_pattern = r"class\s+(\w+)\s*:";
                                    if let Ok(re) = regex::Regex::new(class_pattern) {
                                        if let Some(captures) = re.captures(&code) {
                                            let name = captures[1].to_string();
                                            if character.name != name {
                                                character.name = name.clone();
                                                if let Some(layout) =
                                                    ui.get_list_layout("Character List")
                                                {
                                                    layout.set_item_text(character.id, name);
                                                    redraw = true;
                                                }
                                            }
                                        }
                                    }
                                    character.source = code;
                                    character.source_debug = "".into();
                                }
                            }
                            ContentContext::ItemTemplate(uuid) => {
                                if let Some(item) = project.items.get_mut(&uuid) {
                                    let class_pattern = r"class\s+(\w+)\s*:";
                                    if let Ok(re) = regex::Regex::new(class_pattern) {
                                        if let Some(captures) = re.captures(&code) {
                                            let name = captures[1].to_string();
                                            if item.name != name {
                                                item.name = name.clone();
                                                if let Some(layout) =
                                                    ui.get_list_layout("Item List")
                                                {
                                                    layout.set_item_text(item.id, name);
                                                    redraw = true;
                                                }
                                            }
                                        }
                                    }
                                    item.source = code;
                                    item.source_debug = "".into();
                                }
                            }
                            ContentContext::Sector(uuid) => {
                                if let Some(map) = project.get_map_mut(server_ctx) {
                                    for sector in map.sectors.iter_mut() {
                                        if sector.creator_id == uuid {
                                            sector
                                                .properties
                                                .set("source", rusterix::Value::Str(code.clone()));
                                            break;
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }
        redraw
    }
}
