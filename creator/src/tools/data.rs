use crate::prelude::*;
use ToolEvent::*;

pub struct DataTool {
    id: TheId,
}

impl Tool for DataTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Data Tool"),
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }
    fn info(&self) -> String {
        str!("Data Tool (a).")
    }
    fn icon_name(&self) -> String {
        str!("database")
    }
    fn accel(&self) -> Option<char> {
        Some('a')
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
                PanelIndices::DataEditor as usize,
            ));

            if let Some(layout) = ui.get_hlayout("Game Tool Params") {
                layout.clear();

                let mut build_button = TheTraybarButton::new(TheId::named("Build"));
                build_button
                    .set_status_text("Build and test the source code. Just for validation. Runtime errors are shown in the Log.");
                build_button.set_text("Build".to_string());
                layout.add_widget(Box::new(build_button));

                let mut build_result = TheText::new(TheId::named("Build Result"));
                build_result.limiter_mut().set_min_width(400);
                build_result.set_text("".to_string());
                layout.add_widget(Box::new(build_result));

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
            TheEvent::StateChanged(id, state) => {
                #[allow(clippy::collapsible_if)]
                if id.name == "Build" && *state == TheWidgetState::Clicked {
                    if let Some(value) = ui.get_widget_value("CodeEdit") {
                        if let Some(code) = value.to_string() {
                            // Compile the code to test for errors.
                            let ri = rusterix::RegionInstance::default();
                            match ri.execute(&code) {
                                Ok(_) => {
                                    ui.set_widget_value(
                                        "Build Result",
                                        ctx,
                                        TheValue::Text("Build OK".into()),
                                    );
                                }
                                Err(err) => {
                                    ui.set_widget_value(
                                        "Build Result",
                                        ctx,
                                        TheValue::Text(format!("Error: {}", err)),
                                    );
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
