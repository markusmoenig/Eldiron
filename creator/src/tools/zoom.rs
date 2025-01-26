use crate::prelude::*;
use ToolEvent::*;

pub struct ZoomTool {
    id: TheId,
}

impl Tool for ZoomTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Zoom Tool"),
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }
    fn info(&self) -> String {
        if cfg!(target_os = "macos") {
            str!("Zoom Tool (Z). Global shortcut: Cmd + '-' / '+'.")
        } else {
            str!("Zoom Tool (Z). Global shortcut: Ctrl + '-' / '+'.")
        }
    }
    fn icon_name(&self) -> String {
        str!("zoom")
    }
    fn accel(&self) -> Option<char> {
        Some('z')
    }

    fn tool_event(
        &mut self,
        tool_event: ToolEvent,
        _tool_context: ToolContext,
        ui: &mut TheUI,
        _ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        if let Activate = tool_event {
            if let Some(layout) = ui.get_hlayout("Game Tool Params") {
                layout.clear();

                // let mut text = TheText::new(TheId::empty());
                // text.set_text("Zoom".to_string());
                // layout.add_widget(Box::new(text));

                let mut zoom = TheTextLineEdit::new(TheId::named("Editor Zoom"));
                zoom.set_value(TheValue::Float(1.0));
                zoom.set_info_text(Some("Zoom".to_string()));
                //zoom.set_default_value(TheValue::Float(1.0));
                zoom.set_range(TheValue::RangeF32(1.0..=5.0));
                zoom.set_continuous(true);
                zoom.limiter_mut().set_max_width(140);
                zoom.set_status_text("Set the camera zoom.");

                if let Some(region) = project.get_region(&server_ctx.curr_region) {
                    zoom.set_value(TheValue::Float(region.zoom));
                }

                layout.add_widget(Box::new(zoom));
                layout.set_reverse_index(Some(1));
            }

            return true;
        } else if let DeActivate = tool_event {
            if let Some(layout) = ui.get_hlayout("Game Tool Params") {
                layout.clear();
            }
            return true;
        }

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
        #[allow(clippy::single_match)]
        match event {
            TheEvent::ValueChanged(id, value) => {
                if id.name == "Editor Zoom" {
                    if let Some(v) = value.to_f32() {
                        if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                            region.zoom = v;
                        }
                        if let Some(layout) = ui.get_rgba_layout("Region Editor") {
                            layout.set_zoom(v);
                            layout.relayout(ctx);
                        }
                    }
                }
            }
            _ => {}
        }

        false
    }
}
