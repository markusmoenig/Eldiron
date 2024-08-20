use crate::prelude::*;
use ToolEvent::*;

pub struct ResizeTool {
    id: TheId,
}

impl Tool for ResizeTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Resize Tool"),
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }
    fn info(&self) -> String {
        str!("Resize Tool (I). Resize the region.")
    }
    fn icon_name(&self) -> String {
        str!("transform")
    }
    fn accel(&self) -> Option<char> {
        Some('i')
    }

    fn tool_event(
        &mut self,
        tool_event: ToolEvent,
        _tool_context: ToolContext,
        ui: &mut TheUI,
        _ctx: &mut TheContext,
        project: &mut Project,
        _server: &mut Server,
        _client: &mut Client,
        server_ctx: &mut ServerContext,
    ) -> bool {
        if let Activate = tool_event {
            if let Some(layout) = ui.get_hlayout("Game Tool Params") {
                layout.clear();

                let mut text = TheText::new(TheId::empty());
                text.set_text("Expand".to_string());
                layout.add_widget(Box::new(text));

                let mut drop_down = TheDropdownMenu::new(TheId::named("Region Expansion Mode"));
                drop_down.add_option("Top / Left".to_string());
                drop_down.add_option("Top / Right".to_string());
                drop_down.add_option("Bottom / Left".to_string());
                drop_down.add_option("Bottom / Right".to_string());
                drop_down.set_status_text(
                    "Size changes will will grow or shrink the region from the given corner.",
                );

                layout.add_widget(Box::new(drop_down));

                let mut hdivider = TheHDivider::new(TheId::empty());
                hdivider.limiter_mut().set_max_width(15);
                layout.add_widget(Box::new(hdivider));

                //layout.add_pair("Grow / Shrink".to_string(), Box::new(drop_down));
                let mut width_edit = TheTextLineEdit::new(TheId::named("Region Width Edit"));
                if let Some(region) = project.get_region(&server_ctx.curr_region) {
                    width_edit.set_value(TheValue::Int(region.width));
                }
                width_edit.set_range(TheValue::RangeI32(1..=100000));
                width_edit.set_status_text("The width of the region in grid units.");
                width_edit.limiter_mut().set_max_width(80);
                layout.add_widget(Box::new(width_edit));

                let mut text = TheText::new(TheId::empty());
                text.set_text("x".to_string());
                layout.add_widget(Box::new(text));

                let mut height_edit = TheTextLineEdit::new(TheId::named("Region Height Edit"));
                if let Some(region) = project.get_region(&server_ctx.curr_region) {
                    height_edit.set_value(TheValue::Int(region.height));
                }
                height_edit.set_range(TheValue::RangeI32(1..=100000));
                height_edit.set_status_text("The height of the region in grid units.");
                height_edit.limiter_mut().set_max_width(80);
                layout.add_widget(Box::new(height_edit));

                let mut hdivider = TheHDivider::new(TheId::empty());
                hdivider.limiter_mut().set_max_width(15);
                layout.add_widget(Box::new(hdivider));

                let mut resize_button = TheTraybarButton::new(TheId::named("Region Resize"));
                resize_button.set_text(str!("Resize"));
                resize_button.set_status_text(
                    "Resizes the region (growing or shrinking it) based on the expansion mode.",
                );
                //resize_button.set_disabled(true);

                layout.add_widget(Box::new(resize_button));

                // if let Some(region) = project.get_region(&server_ctx.curr_region) {
                //     //zoom.set_value(TheValue::Float(region.zoom));
                //     let mut text = TheText::new(TheId::empty());
                //     text.set_text(format!("{}x{}"));
                //     layout.add_widget(Box::new(text));
                // }

                //layout.set_reverse_index(Some(1));
            }

            return true;
        } else if let DeActivate = tool_event {
            if let Some(layout) = ui.get_hlayout("Game Tool Params") {
                layout.clear();
                layout.set_reverse_index(None);
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
            // TheEvent::ValueChanged(id, value) => {
            //     if id.name == "Region Width Edit" {
            //         if let Some(width) = value.to_f32() {
            //             if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
            //             }
            //         }
            //     }
            // }
            TheEvent::StateChanged(id, TheWidgetState::Clicked) => {
                if id.name == "Region Resize" {
                    let new_width = ui
                        .get_widget_value("Region Width Edit")
                        .unwrap()
                        .to_i32()
                        .unwrap();

                    let new_height = ui
                        .get_widget_value("Region Height Edit")
                        .unwrap()
                        .to_i32()
                        .unwrap();

                    let expansion_mode = ui
                        .get_widget_value("Region Expansion Mode")
                        .unwrap()
                        .to_i32()
                        .unwrap();

                    //println!("{} {} {}", expansion_mode, new_width, new_height);

                    if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {}
                }
            }
            _ => {}
        }

        false
    }
}
