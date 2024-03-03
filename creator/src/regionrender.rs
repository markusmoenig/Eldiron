use crate::prelude::*;

pub struct RegionRender {}

#[allow(clippy::new_without_default)]
impl RegionRender {
    pub fn new() -> Self {
        Self {}
    }

    /// Build the UI
    pub fn build(&self) -> TheCanvas {
        let mut canvas = TheCanvas::new();

        // Toolbar
        let mut toolbar_canvas = TheCanvas::default();
        let traybar_widget = TheTraybar::new(TheId::empty());
        toolbar_canvas.set_widget(traybar_widget);
        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.set_background_color(None);

        toolbar_hlayout.set_margin(vec4i(10, 1, 5, 1));
        toolbar_hlayout.set_padding(5);

        // let mut origin_text = TheText::new(TheId::empty());
        // origin_text.set_text("Origin Offset".to_string());
        // toolbar_hlayout.add_widget(Box::new(origin_text));

        // create_float3_widgets(
        //     &mut toolbar_hlayout,
        //     TheId::named("Camera RO"),
        //     Vec3f::zero(),
        //     vec!["X", "Y", "Z"],
        // );

        // let mut center_text = TheText::new(TheId::empty());
        // center_text.set_text("Center".to_string());
        // toolbar_hlayout.add_widget(Box::new(center_text));

        // create_float3_widgets(
        //     &mut toolbar_hlayout,
        //     TheId::named("Camera RD"),
        //     Vec3f::zero(),
        //     vec!["X", "Y", "Z"],
        // );

        toolbar_canvas.set_layout(toolbar_hlayout);

        // Camera
        let mut camera_canvas = TheCanvas::default();

        let mut vlayout = TheVLayout::new(TheId::named("Camera Layout"));
        vlayout.set_margin(vec4i(5, 5, 5, 10));
        vlayout.set_alignment(TheHorizontalAlign::Left);
        vlayout.limiter_mut().set_max_width(150);

        let mut drop_down = TheDropdownMenu::new(TheId::named("Camera Type"));
        drop_down.add_option("Pinhole".to_string());
        drop_down.add_option("Orthogonal".to_string());
        drop_down.set_disabled(true);

        let mut text = TheText::new(TheId::empty());
        text.set_text_size(12.0);
        text.set_text("Camera Type".to_string());
        //vlayout.add_widget(Box::new(text));
        vlayout.add_widget(Box::new(drop_down));

        let mut camera_origin_x = TheTextLineEdit::new(TheId::named("Camera Origin X"));
        camera_origin_x.limiter_mut().set_max_width(130);
        camera_origin_x.set_range(TheValue::RangeF32(-5.0..=5.0));
        camera_origin_x.set_continuous(true);
        camera_origin_x.set_disabled(true);

        let mut camera_origin_y = TheTextLineEdit::new(TheId::named("Camera Origin Y"));
        camera_origin_y.limiter_mut().set_max_width(130);
        camera_origin_y.set_range(TheValue::RangeF32(0.0..=5.0));
        camera_origin_y.set_continuous(true);
        camera_origin_y.set_disabled(true);

        let mut camera_origin_z = TheTextLineEdit::new(TheId::named("Camera Origin Z"));
        camera_origin_z.limiter_mut().set_max_width(130);
        camera_origin_z.set_range(TheValue::RangeF32(-5.0..=5.0));
        camera_origin_z.set_continuous(true);
        camera_origin_z.set_disabled(true);

        let mut text = TheText::new(TheId::empty());
        text.set_text_size(12.0);
        text.set_text("Camera Origin Offset".to_string());
        vlayout.add_widget(Box::new(text));
        vlayout.add_widget(Box::new(camera_origin_x));
        vlayout.add_widget(Box::new(camera_origin_y));
        vlayout.add_widget(Box::new(camera_origin_z));

        // ---

        let mut camera_center_x = TheTextLineEdit::new(TheId::named("Camera Center X"));
        camera_center_x.limiter_mut().set_max_width(130);
        camera_center_x.set_range(TheValue::RangeF32(-5.0..=5.0));
        camera_center_x.set_continuous(true);
        camera_center_x.set_disabled(true);

        let mut camera_center_y = TheTextLineEdit::new(TheId::named("Camera Center Y"));
        camera_center_y.limiter_mut().set_max_width(130);
        camera_center_y.set_range(TheValue::RangeF32(0.0..=5.0));
        camera_center_y.set_continuous(true);
        camera_center_y.set_disabled(true);

        let mut camera_center_z = TheTextLineEdit::new(TheId::named("Camera Center Z"));
        camera_center_z.limiter_mut().set_max_width(130);
        camera_center_z.set_range(TheValue::RangeF32(-5.0..=5.0));
        camera_center_z.set_continuous(true);
        camera_center_z.set_disabled(true);

        let mut text = TheText::new(TheId::empty());
        text.set_text_size(12.0);
        text.set_text("Camera Center".to_string());
        vlayout.add_widget(Box::new(text));
        vlayout.add_widget(Box::new(camera_center_x));
        vlayout.add_widget(Box::new(camera_center_y));
        vlayout.add_widget(Box::new(camera_center_z));

        camera_canvas.set_layout(vlayout);

        //

        //canvas.set_top(toolbar_canvas);
        canvas.set_right(camera_canvas);

        canvas
    }

    pub fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server: &mut Server,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let mut redraw = false;

        match event {
            TheEvent::IndexChanged(id, index) => {
                if id.name == "Camera Type" {
                    if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                        region.camera_type = match index {
                            1 => CameraType::Orthogonal,
                            _ => CameraType::Pinhole,
                        };
                        server.update_region(region);
                    }
                }
            }
            TheEvent::ValueChanged(id, value) => {
                if id.name == "Camera Origin X" {
                    if let Some(v) = value.as_f32() {
                        if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                            region.camera_origin_offset.x = v;
                            server.update_region(region);
                        }
                    }
                } else if id.name == "Camera Origin Y" {
                    if let Some(v) = value.as_f32() {
                        if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                            region.camera_origin_offset.y = v;
                            server.update_region(region);
                        }
                    }
                } else if id.name == "Camera Origin Z" {
                    if let Some(v) = value.as_f32() {
                        if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                            region.camera_origin_offset.z = v;
                            server.update_region(region);
                        }
                    }
                } else if id.name == "Camera Center X" {
                    if let Some(v) = value.as_f32() {
                        if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                            region.camera_center_offset.x = v;
                            server.update_region(region);
                        }
                    }
                } else if id.name == "Camera Center Y" {
                    if let Some(v) = value.as_f32() {
                        if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                            region.camera_center_offset.y = v;
                            server.update_region(region);
                        }
                    }
                } else if id.name == "Camera Center Z" {
                    if let Some(v) = value.as_f32() {
                        if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                            region.camera_center_offset.z = v;
                            server.update_region(region);
                        }
                    }
                }
            }
            _ => {}
        }

        redraw
    }
}
