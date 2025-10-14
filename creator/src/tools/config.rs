use crate::prelude::*;
use ToolEvent::*;

use crate::editor::{CONFIG, CONFIGEDITOR};

pub struct ConfigTool {
    id: TheId,
}

impl Tool for ConfigTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Config Tool"),
        }
    }
    fn id(&self) -> TheId {
        self.id.clone()
    }
    fn info(&self) -> String {
        str!("Config Tool.")
    }
    fn icon_name(&self) -> String {
        str!("gear")
    }
    fn accel(&self) -> Option<char> {
        None //Some('x')
    }

    fn tool_event(
        &mut self,
        tool_event: ToolEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        match tool_event {
            Activate => {
                ctx.ui.send(TheEvent::SetStackIndex(
                    TheId::named("Main Stack"),
                    PanelIndices::ConfigEditor as usize,
                ));

                ui.set_widget_value("ConfigEdit", ctx, TheValue::Text(project.config.clone()));
                server_ctx.curr_map_tool_type = MapToolType::General;

                true
            }
            DeActivate => true,
            _ => false,
        }
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        _server_ctx: &mut ServerContext,
    ) -> bool {
        let redraw = false;
        #[allow(clippy::single_match)]
        match event {
            TheEvent::ValueChanged(id, value) => {
                if id.name == "ConfigEdit" {
                    if let Some(config_string) = value.to_string() {
                        project.config = config_string;
                        if let Ok(toml) = project.config.parse::<Table>() {
                            *CONFIG.write().unwrap() = toml;
                        }
                        let ts = CONFIGEDITOR.read().unwrap().tile_size;
                        CONFIGEDITOR.write().unwrap().read_defaults();

                        // If the tile_size changed update the materials
                        if ts != CONFIGEDITOR.read().unwrap().tile_size {
                            ctx.ui.send(TheEvent::Custom(
                                TheId::named("Update Materialpicker"),
                                TheValue::Empty,
                            ));
                        }
                    }
                }
            }
            _ => {}
        }
        redraw
    }
}
