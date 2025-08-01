use crate::prelude::*;
use ToolEvent::*;

pub struct CodeTool {
    id: TheId,
}

impl Tool for CodeTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Code Tool"),
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }
    fn info(&self) -> String {
        str!("Code Tool (Shift + C).")
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
        _tool_context: ToolContext,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        _project: &mut Project,
        _server_ctx: &mut ServerContext,
    ) -> bool {
        if let Activate = tool_event {
            // ctx.ui.send(TheEvent::Custom(
            //     TheId::named("Set CodeGrid Panel"),
            //     TheValue::Empty,
            // ));

            ctx.ui.send(TheEvent::SetStackIndex(
                TheId::named("Main Stack"),
                PanelIndices::CodeEditor as usize,
            ));

            if let Some(layout) = ui.get_hlayout("Game Tool Params") {
                layout.clear();

                let mut build_button = TheTraybarButton::new(TheId::named("Build"));
                build_button
                    .set_status_text("Build and test the source code. Just for validation. Runtime errors are shown in the Log.");
                build_button.set_text("Build".to_string());
                layout.add_widget(Box::new(build_button));

                let mut spaces_switch = TheGroupButton::new(TheId::named("Code Spaces Switch"));
                spaces_switch.add_text_status(
                    "Show Spaces".to_string(),
                    str!("Visually display spaces in the editor."),
                );
                spaces_switch.add_text_status("Hide".to_string(), "Hide spaces.".to_string());
                spaces_switch.set_index(0);
                spaces_switch.set_item_width(100);
                layout.add_widget(Box::new(spaces_switch));

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
        #[allow(clippy::single_match)]
        match event {
            TheEvent::IndexChanged(id, index) => {
                if id.name == "Code Spaces Switch" {
                    if let Some(edit) = ui.get_text_area_edit("CodeEdit") {
                        edit.as_code_editor(
                            "Python",
                            TheCodeEditorSettings {
                                auto_bracket_completion: true,
                                auto_indent: true,
                                indicate_space: *index == 0,
                            },
                        );
                    }
                }
            }
            TheEvent::StateChanged(id, state) => {
                #[allow(clippy::collapsible_if)]
                if id.name == "Build" && *state == TheWidgetState::Clicked {
                    if let Some(value) = ui.get_widget_value("CodeEdit") {
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
            TheEvent::ValueChanged(id, value) => {
                if id.name == "CodeEdit" {
                    if let Some(code) = value.to_string() {
                        // println!("cc {:?}", server_ctx.cc);
                        match server_ctx.cc {
                            ContentContext::CharacterInstance(uuid) => {
                                if let Some(region) =
                                    project.get_region_mut(&server_ctx.curr_region)
                                {
                                    if let Some(character_instance) =
                                        region.characters.get_mut(&uuid)
                                    {
                                        character_instance.source = code;
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
