use crate::prelude::*;
use ToolEvent::*;

use crate::editor::TILEFXEDITOR;

pub struct FXTool {
    id: TheId,

    edit_mode_index: i32,
}

impl Tool for FXTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("FX Tool"),

            edit_mode_index: 1,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }
    fn info(&self) -> String {
        str!("FX Tool (X). Apply effects to tiles.")
    }
    fn icon_name(&self) -> String {
        str!("magicwand")
    }
    fn accel(&self) -> Option<char> {
        Some('x')
    }

    fn tool_event(
        &mut self,
        tool_event: ToolEvent,
        _tool_context: ToolContext,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server: &mut Server,
        _client: &mut Client,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let coord = match tool_event {
            TileDown(c, _) => c,
            TileDrag(c, _) => c,
            Activate => {
                ctx.ui
                    .send(TheEvent::SetStackIndex(TheId::named("Main Stack"), 3));
                server_ctx.show_fx_marker = true;

                if let Some(layout) = ui.get_hlayout("Game Tool Params") {
                    layout.clear();

                    // Material Group
                    let mut gb = TheGroupButton::new(TheId::named("Effects Mode Group"));
                    gb.add_text_status(str!("Edit"), str!("Edit the effects of existing tiles."));
                    gb.add_text_status(str!("Add"), str!("Add the current effects to new tiles."));
                    gb.set_item_width(85);

                    gb.set_index(self.edit_mode_index);

                    layout.add_widget(Box::new(gb));
                }

                return true;
            }
            DeActivate => {
                server_ctx.show_fx_marker = false;
                return true;
            }
            _ => {
                return false;
            }
        };

        let fx_coord = vec3i(coord.x, 0, coord.y);

        if self.edit_mode_index == 0 {
            // Edit
        } else {
            // Add

            let object = TILEFXEDITOR.lock().unwrap().object.clone();
            if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                region.effects.insert(fx_coord, object);
                server.update_region(region);
            }
        }

        true
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        _project: &mut Project,
        _server: &mut Server,
        _client: &mut Client,
        _server_ctx: &mut ServerContext,
    ) -> bool {
        #[allow(clippy::single_match)]
        match &event {
            TheEvent::IndexChanged(id, index) => {
                if id.name == "Effects Mode Group" {
                    self.edit_mode_index = *index as i32;
                }
            }
            _ => {}
        }
        false
    }
}
