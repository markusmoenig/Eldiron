use crate::prelude::*;
use ToolEvent::*;

use crate::editor::SIDEBARMODE;

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
        str!("Code Tool (C).")
    }
    fn icon_name(&self) -> String {
        str!("code")
    }
    fn accel(&self) -> Option<char> {
        Some('c')
    }

    fn tool_event(
        &mut self,
        tool_event: ToolEvent,
        _tool_context: ToolContext,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        _project: &mut Project,
        _server: &mut Server,
        _client: &mut Client,
        _server_ctx: &mut ServerContext,
    ) -> bool {
        if let Activate = tool_event {
            // ctx.ui.send(TheEvent::Custom(
            //     TheId::named("Set CodeGrid Panel"),
            //     TheValue::Empty,
            // ));

            ctx.ui.send(TheEvent::SetStackIndex(
                TheId::named("Main Stack"),
                PanelIndices::TextEditor as usize,
            ));

            if let Some(layout) = ui.get_hlayout("Game Tool Params") {
                layout.clear();

                let mut compile_button = TheTraybarButton::new(TheId::named("Compile"));
                compile_button.set_status_text("Compile the source.");
                compile_button.set_text("Compile".to_string());
                layout.add_widget(Box::new(compile_button));
            }

            return true;
        };

        false
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        _ctx: &mut TheContext,
        project: &mut Project,
        _server: &mut shared::server::Server,
        _client: &mut shared::client::Client,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let mut redraw = false;
        #[allow(clippy::single_match)]
        match event {
            TheEvent::StateChanged(id, state) => {
                #[allow(clippy::collapsible_if)]
                if id.name == "Compile" && *state == TheWidgetState::Clicked {
                    if let Some(value) = ui.get_widget_value("CodeEdit") {
                        if let Some(code) = value.to_string() {
                            if *SIDEBARMODE.lock().unwrap() == SidebarMode::Character {
                                // Character mode

                                if let Some(character_id) = server_ctx.curr_character {
                                    if let Some(character) =
                                        project.characters.get_mut(&character_id)
                                    {
                                        // Extract the character class name
                                        let class_pattern = r"class\s+(\w+)\s*\(\s*Entity\s*\)";
                                        let re = regex::Regex::new(class_pattern).unwrap();
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

                                        // Compile the code to test for errors.
                                        let mut ri = rusterix::RegionInstance::default();
                                        ri.apply_base_classes();
                                        match ri.execute(&code) {
                                            Ok(_) => {
                                                println!("OK");
                                            }
                                            Err(err) => {
                                                println!("Error: {}", err);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            TheEvent::ValueChanged(id, value) => {
                if id.name == "CodeEdit" {
                    if let Some(code) = value.to_string() {
                        if *SIDEBARMODE.lock().unwrap() == SidebarMode::Region {
                            // Region mode, check the currently selected region content
                            if let Some(region_content_id) = server_ctx.curr_region_content {
                                if let Some(region) =
                                    project.get_region_mut(&server_ctx.curr_region)
                                {
                                    if let Some(character_instance) =
                                        region.characters.get_mut(&region_content_id)
                                    {
                                        character_instance.source = code;
                                    }
                                }
                            }
                        } else if *SIDEBARMODE.lock().unwrap() == SidebarMode::Character {
                            // Character mode, store the code in the current character
                            if let Some(character_id) = server_ctx.curr_character {
                                if let Some(character) = project.characters.get_mut(&character_id) {
                                    character.source = code;
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        redraw
    }
}
