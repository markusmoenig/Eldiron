use crate::prelude::*;
use ToolEvent::*;

pub struct ScreenPickerTool {
    id: TheId,
}

impl Tool for ScreenPickerTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Picker Tool"),
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }
    fn info(&self) -> String {
        str!("Picker Tool (K). Selects the widget at the click position.")
    }
    fn icon_name(&self) -> String {
        str!("picker")
    }
    fn accel(&self) -> Option<char> {
        Some('k')
    }

    fn tool_event(
        &mut self,
        tool_event: ToolEvent,
        _tool_context: ToolContext,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        _server: &mut Server,
        client: &mut Client,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let coord = match tool_event {
            TileDown(c, _) => c,
            TileDrag(c, _) => c,
            Activate => {
                return true;
            }
            _ => {
                return false;
            }
        };

        if let Some(screen) = project.screens.get_mut(&server_ctx.curr_screen) {
            let sorted_widgets = screen.sorted_widgets_by_size();
            for widget in sorted_widgets.iter() {
                if widget.is_inside(&coord) {
                    if let Some(layout) = ui.get_list_layout("Screen Content List") {
                        layout.select_item(widget.id, ctx, true);
                    }
                    /*else if self.editor_mode == ScreenEditorMode::Erase {
                    open_delete_confirmation_dialog(
                        "Delete Widget ?",
                        format!("Permanently delete '{}' ?", widget.name).as_str(),
                        widget.id,
                        ui,
                        ctx,
                        );
                        }*/
                }
            }
        }

        // TODO: MOVE THIS TO A SEPARATE INTERACTION TOOL
        if let Some(screen) = project.screens.get_mut(&server_ctx.curr_screen) {
            client.touch_down(
                &server_ctx.curr_screen,
                vec2i(coord.x * screen.grid_size, coord.y * screen.grid_size),
            );
        }

        true
    }
}
