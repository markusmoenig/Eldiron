use crate::editor::{CUSTOMCAMERA, RUSTERIX};
use crate::prelude::*;
use shared::prelude::*;

use rusterix::ValueContainer;

pub enum RenderMoveAction {
    Forward,
    Backward,
    Left,
    Right,
}

pub struct RenderEditor {
    drag_coord: Vec2<i32>,

    pub first_draw: bool,
    edited: bool,

    accum: i32,
}

#[allow(clippy::new_without_default)]
impl RenderEditor {
    pub fn new() -> Self {
        Self {
            drag_coord: Vec2::zero(),

            first_draw: true,
            edited: false,

            accum: 0,
        }
    }

    pub fn build_trace_canvas(&mut self) -> TheCanvas {
        let mut center = TheCanvas::new();

        let mut text_layout = TheTextLayout::new(TheId::named("Brush Settings"));

        let mut trace_button = TheTraybarButton::new(TheId::named("Trace Button"));
        trace_button.set_text("Start Trace".into());
        trace_button.limiter_mut().set_min_width(120);
        text_layout.add_pair("Trace".into(), Box::new(trace_button));

        center.set_layout(text_layout);

        /*
        let mut preview_canvas: TheCanvas = TheCanvas::new();
        let mut render_view = TheRenderView::new(TheId::named("Brush Preview"));
        render_view.limiter_mut().set_max_size(Vec2::new(300, 300));
        preview_canvas.set_widget(render_view);

        center.set_right(preview_canvas);*/

        center
    }

    pub fn draw(
        &mut self,
        ui: &mut TheUI,
        _ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
        build_values: &mut ValueContainer,
    ) {
        if let Some(render_view) = ui.get_render_view("PolyView") {
            let dim = *render_view.dim();

            let buffer = render_view.render_buffer_mut();
            buffer.resize(dim.width, dim.height);

            if let Some(region) = project.get_region_ctx_mut(server_ctx) {
                CUSTOMCAMERA
                    .write()
                    .unwrap()
                    .update_action(region, server_ctx);
                CUSTOMCAMERA
                    .write()
                    .unwrap()
                    .update_camera(region, server_ctx);

                let mut rusterix = RUSTERIX.write().unwrap();
                if self.first_draw {
                    rusterix.build_scene_d3(&region.map, build_values);
                    rusterix.build_terrain_d3(&mut region.map, &ValueContainer::default());
                    self.first_draw = false;
                }

                rusterix.client.scene_d3.dynamic_lights = vec![];

                if server_ctx.curr_render_tool_helper != RenderToolHelper::Tracer {
                    rusterix.client.draw_d3(
                        &region.map,
                        buffer.pixels_mut(),
                        dim.width as usize,
                        dim.height as usize,
                    );
                } else {
                    rusterix.client.trace(
                        buffer.pixels_mut(),
                        dim.width as usize,
                        dim.height as usize,
                        self.accum,
                    );
                    self.accum += 1;
                }
            }
        }
    }

    pub fn map_event(
        &mut self,
        map_event: MapEvent,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        _map: &mut Map,
        _server_ctx: &mut ServerContext,
    ) -> Option<RegionUndoAtom> {
        match &map_event {
            MapEvent::MapClicked(coord) => {
                self.drag_coord = *coord;
                self.edited = false;
            }
            MapEvent::MapUp(_coord) => {}
            _ => {}
        }

        None
    }

    pub fn start_trace(&mut self) {
        self.accum = 0;
    }

    pub fn reset_trace(&mut self) {
        self.accum = 0;
    }

    pub fn scroll_by(
        &mut self,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        _server_ctx: &mut ServerContext,
        _coord: Vec2<i32>,
    ) {
        // self.orbit_camera.zoom(coord.y as f32);
    }
}
