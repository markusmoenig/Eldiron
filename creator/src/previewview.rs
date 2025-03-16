use crate::editor::RUSTERIX;
use crate::prelude::*;
use rusterix::{D3Camera, D3FirstPCamera, D3IsoCamera, ValueContainer};

pub struct PreviewView {
    pub camera: MapCamera,
}

#[allow(clippy::new_without_default)]
impl PreviewView {
    pub fn new() -> Self {
        Self {
            camera: MapCamera::ThreeDFirstPerson,
        }
    }

    pub fn build(&mut self) -> TheCanvas {
        let mut center = TheCanvas::new();

        let view = TheRenderView::new(TheId::named("PreviewView"));
        center.set_widget(view);

        center
    }

    pub fn draw(
        &mut self,
        region: &Region,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        _server_ctx: &mut ServerContext,
    ) {
        let mut build_values = ValueContainer::default();
        extract_build_values_from_config(&mut build_values);

        if let Some(render_view) = ui.get_render_view("PreviewView") {
            let dim = *render_view.dim();

            let buffer = render_view.render_buffer_mut();
            buffer.resize(dim.width, dim.height);

            let mut rusterix = RUSTERIX.write().unwrap();

            if self.camera == MapCamera::ThreeDIso {
                let p = Vec3::new(
                    region.editing_position_3d.x,
                    0.0,
                    region.editing_position_3d.z,
                );
                let look_at = Vec3::new(
                    region.editing_look_at_3d.x,
                    0.0,
                    region.editing_look_at_3d.z,
                );

                let raw_direction = p - look_at;
                let distance = raw_direction.magnitude() * 2.0;
                let dir_norm = raw_direction.normalized();

                let yaw = dir_norm.z.atan2(dir_norm.x);
                let pitch = 45.0_f32.to_radians();

                let rot_yaw = Mat4::<f32>::rotation_y(yaw);
                let rot_pitch = Mat4::<f32>::rotation_x(-pitch);
                let rotation = rot_yaw * rot_pitch;

                let base_offset = Vec4::new(0.0, 0.0, distance, 0.0);
                let rotated_direction = (rotation * base_offset).xyz();
                let final_camera_pos = look_at + rotated_direction;

                rusterix.client.camera_d3 = Box::new(D3IsoCamera::new());

                rusterix
                    .client
                    .camera_d3
                    .set_parameter_vec3("center", look_at);
                rusterix
                    .client
                    .camera_d3
                    .set_parameter_vec3("position", final_camera_pos);

                /*
                rusterix.client.camera_d3.set_parameter_vec3("center", p);
                rusterix
                    .client
                    .camera_d3
                    .set_parameter_vec3("position", p + vek::Vec3::new(-10.0, 10.0, 10.0));
                */
            } else if self.camera == MapCamera::ThreeDFirstPerson {
                let p = Vec3::new(
                    region.editing_position_3d.x,
                    1.5,
                    region.editing_position_3d.z,
                );
                let look_at = Vec3::new(
                    region.editing_look_at_3d.x,
                    1.5,
                    region.editing_look_at_3d.z,
                );

                rusterix.client.camera_d3 = Box::new(D3FirstPCamera::new());
                rusterix.client.camera_d3.set_parameter_vec3("position", p);
                rusterix
                    .client
                    .camera_d3
                    .set_parameter_vec3("center", look_at);
            }

            rusterix.build_scene_d3(&region.map, &build_values);
            let assets = rusterix.assets.clone();
            rusterix
                .client
                .apply_entities_items_d3(&region.map, &assets);
            rusterix
                .client
                .draw_d3(buffer.pixels_mut(), dim.width as usize, dim.height as usize);

            // Hud
            let bg_color = [50, 50, 50, 255];
            let text_color = [150, 150, 150, 255];
            let stride = buffer.stride();

            ctx.draw
                .rect(buffer.pixels_mut(), &(0, 0, 75, 20), stride, &bg_color);

            let text = if self.camera == MapCamera::ThreeDIso {
                "Iso"
            } else {
                "FirstP"
            };

            if let Some(font) = &ctx.ui.font {
                ctx.draw.text_rect(
                    buffer.pixels_mut(),
                    &(0, 0, 75, 20),
                    stride,
                    font,
                    13.0,
                    text,
                    &text_color,
                    &bg_color,
                    TheHorizontalAlign::Center,
                    TheVerticalAlign::Center,
                );
            }
        }
    }

    /*
    pub fn handle_event(
        &mut self,
        event: &TheEvent,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        _project: &mut Project,
        _server_ctx: &mut ServerContext,
    ) -> bool {
        let redraw = false;
        #[allow(clippy::single_match)]
        match event {
            TheEvent::RenderViewClicked(id, coord) => {
                if id.name == "PreviewView" && coord.x < 75 && coord.y < 20 {
                    if self.camera == MapCamera::ThreeDIso {
                        self.camera = MapCamera::ThreeDFirstPerson;
                    } else {
                        self.camera = MapCamera::ThreeDIso;
                    }
                }
            }
            _ => {}
        }

        redraw
    }*/
}
