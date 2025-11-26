use crate::editor::EDITCAMERA;
use crate::prelude::*;
use shared::prelude::*;

use rusterix::{AccumBuffer, Rusterix};

pub enum RenderMoveAction {
    Forward,
    Backward,
    Left,
    Right,
}

pub struct RenderEditor {
    drag_coord: Vec2<i32>,
    edited: bool,

    accum_buffer: AccumBuffer,
}

#[allow(clippy::new_without_default)]
impl RenderEditor {
    pub fn new() -> Self {
        Self {
            drag_coord: Vec2::zero(),
            edited: false,

            accum_buffer: AccumBuffer::empty(),
        }
    }

    pub fn build_trace_canvas(&mut self) -> TheCanvas {
        let mut center = TheCanvas::new();

        let mut text_layout = TheTextLayout::new(TheId::named("Brush Settings"));

        let mut trace_button = TheTraybarButton::new(TheId::named("Trace Button"));
        trace_button.set_text(fl!("render_editor_trace_button"));
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
        render_view: &mut dyn TheRenderViewTrait,
        _ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
        rusterix: &mut Rusterix,
    ) {
        let dim = *render_view.dim();

        let buffer = render_view.render_buffer_mut();
        buffer.resize(dim.width, dim.height);

        if let Some(region) = project.get_region_ctx_mut(server_ctx) {
            EDITCAMERA
                .write()
                .unwrap()
                .update_action(region, server_ctx);
            EDITCAMERA
                .write()
                .unwrap()
                .update_camera(region, server_ctx, rusterix);

            {
                rusterix.client.scene.dynamic_lights = vec![];
                rusterix.build_entities_items_d3(&region.map);

                if server_ctx.curr_render_tool_helper != RenderToolHelper::Tracer {
                    rusterix.draw_d3(
                        &region.map,
                        buffer.pixels_mut(),
                        dim.width as usize,
                        dim.height as usize,
                    );
                } else {
                    if dim.width as usize != self.accum_buffer.width
                        || dim.height as usize != self.accum_buffer.height
                    {
                        self.accum_buffer =
                            AccumBuffer::new(dim.width as usize, dim.height as usize);
                    }
                    rusterix.trace_scene(&mut self.accum_buffer);
                    self.accum_buffer.convert_to_u8(buffer.pixels_mut());
                }
            }

            // if let Ok(mut toollist) = TOOLLIST.try_write() {
            //     toollist.draw_hud(buffer, &mut region.map, ctx, server_ctx, &rusterix.assets);
            // }
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
        self.accum_buffer.reset();
    }

    pub fn reset_trace(&mut self) {
        self.accum_buffer.reset();
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
