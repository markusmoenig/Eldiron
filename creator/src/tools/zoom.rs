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
        str!("Zoom Tool (Z).")
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
        _project: &mut Project,
        _server: &mut Server,
        _client: &mut Client,
        _server_ctx: &mut ServerContext,
    ) -> bool {
        if let Activate = tool_event {
            if let Some(layout) = ui.get_hlayout("Game Tool Params") {
                layout.clear();

                let mut text = TheText::new(TheId::empty());
                text.set_text("Zoom".to_string());
                layout.add_widget(Box::new(text));

                let mut zoom = TheTextLineEdit::new(TheId::named("Editor Zoom"));
                zoom.set_value(TheValue::Float(1.0));
                //zoom.set_default_value(TheValue::Float(1.0));
                zoom.set_range(TheValue::RangeF32(1.0..=5.0));
                zoom.set_continuous(true);
                zoom.limiter_mut().set_max_width(140);
                zoom.set_status_text("Set the camera zoom.");

                layout.add_widget(Box::new(zoom));
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
        server: &mut Server,
        _client: &mut Client,
        server_ctx: &mut ServerContext,
    ) -> bool {
        #[allow(clippy::single_match)]
        match event {
            TheEvent::ValueChanged(id, value) => {
                if id.name == "Editor Zoom" {
                    if let Some(v) = value.to_f32() {
                        if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                            server.set_zoom(region.id, v);
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